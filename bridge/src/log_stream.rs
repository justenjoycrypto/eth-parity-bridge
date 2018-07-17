use error::{self, ResultExt};
use ethabi;
use futures::future::FromErr;
use futures::{Async, Future, Poll, Stream};
use std::time::Duration;
use tokio_timer::{Interval, Timeout, Timer};
use web3;
use web3::api::Namespace;
use web3::helpers::CallResult;
use web3::types::{Address, FilterBuilder, H256, Log, U256};
use web3::Transport;

fn web3_topic(topic: ethabi::Topic<ethabi::Hash>) -> Option<Vec<H256>> {
    let t: Vec<ethabi::Hash> = topic.into();
    // parity does not conform to an ethereum spec
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

fn web3_filter(filter: ethabi::TopicFilter, address: Address) -> FilterBuilder {
    let t0 = web3_topic(filter.topic0);
    let t1 = web3_topic(filter.topic1);
    let t2 = web3_topic(filter.topic2);
    let t3 = web3_topic(filter.topic3);
    FilterBuilder::default()
        .address(vec![address])
        .topics(t0, t1, t2, t3)
}

/// options for creating a `LogStream`. passed to `LogStream::new`
pub struct LogStreamOptions<T> {
    pub filter: ethabi::TopicFilter,
    pub request_timeout: Duration,
    pub poll_interval: Duration,
    pub confirmations: u32,
    pub transport: T,
    pub contract_address: Address,
    pub after: u64,
}

/// Contains all logs matching `LogStream` filter in inclusive block range `[from, to]`.
#[derive(Debug, PartialEq)]
pub struct LogsInBlockRange {
    pub from: u64,
    pub to: u64,
    pub logs: Vec<Log>,
}

/// Log Stream state.
enum State<T: Transport> {
    /// Log Stream is waiting for timer to poll.
    AwaitInterval,
    /// Fetching best block number.
    AwaitBlockNumber(Timeout<FromErr<CallResult<U256, T::Out>, error::Error>>),
    /// Fetching logs for new best block.
    AwaitLogs {
        from: u64,
        to: u64,
        future: Timeout<FromErr<CallResult<Vec<Log>, T::Out>, error::Error>>,
    },
}

/// `futures::Stream` that fetches logs from `contract_address` matching `filter`
/// with adjustable `poll_interval` and `request_timeout`.
/// yields new logs that are `confirmations` blocks deep
pub struct LogStream<T: Transport> {
    request_timeout: Duration,
    confirmations: u32,
    transport: T,
    last_checked_block: u64,
    timer: Timer,
    poll_interval: Interval,
    state: State<T>,
    filter: FilterBuilder,
}

impl<T: Transport> LogStream<T> {
    /// creates a `LogStream`
    pub fn new(options: LogStreamOptions<T>) -> Self {
        let timer = Timer::default();
        LogStream {
            request_timeout: options.request_timeout,
            confirmations: options.confirmations,
            poll_interval: timer.interval(options.poll_interval),
            transport: options.transport,
            last_checked_block: options.after,
            timer: timer,
            state: State::AwaitInterval,
            filter: web3_filter(options.filter, options.contract_address),
        }
    }
}

impl<T: Transport> Stream for LogStream<T> {
    type Item = LogsInBlockRange;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let (next_state, value_to_yield) = match self.state {
                State::AwaitInterval => {
                    // wait until `interval` has passed
                    let _ = try_stream!(
                        self.poll_interval
                            .poll()
                            .chain_err(|| "LogStream: polling interval failed")
                    );
                    trace!("LogStream: polling last block number");
                    let future = web3::api::Eth::new(&self.transport).block_number();
                    let next_state = State::AwaitBlockNumber(
                        self.timer.timeout(future.from_err(), self.request_timeout),
                    );
                    (next_state, None)
                }
                State::AwaitBlockNumber(ref mut future) => {
                    let last_block = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "LogStream: fetching of last block number failed")
                    ).as_u64();
                    trace!("LogStream: fetched last block number {}", last_block);
                    // subtraction that saturates at zero
                    let last_confirmed_block = last_block.saturating_sub(self.confirmations as u64);

                    let next_state = if self.last_checked_block < last_confirmed_block {
                        let from = self.last_checked_block + 1;
                        let filter = self.filter
                            .clone()
                            .from_block(from.into())
                            .to_block(last_confirmed_block.into())
                            .build();
                        let future = web3::api::Eth::new(&self.transport).logs(filter);

                        State::AwaitLogs {
                            from: from,
                            to: last_confirmed_block,
                            future: self.timer.timeout(future.from_err(), self.request_timeout),
                        }
                    } else {
                        trace!("LogStream: no blocks confirmed since we last checked. waiting some more");
                        State::AwaitInterval
                    };

                    (next_state, None)
                }
                State::AwaitLogs {
                    ref mut future,
                    from,
                    to,
                } => {
                    let logs = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "LogStream: polling web3 logs failed")
                    );
                    let log_range_to_yield = LogsInBlockRange { from, to, logs };

                    self.last_checked_block = to;
                    (State::AwaitInterval, Some(log_range_to_yield))
                }
            };

            self.state = next_state;

            if value_to_yield.is_some() {
                return Ok(Async::Ready(value_to_yield));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::home::HomeBridge;
    use helpers::StreamExt;
    use rustc_hex::FromHex;
    use tokio_core::reactor::Core;
    use web3::types::{Bytes, Log};

    #[test]
    fn test_log_stream_twice_no_logs() {
        let deposit_topic = HomeBridge::default()
            .events()
            .deposit()
            .create_filter()
            .topic0;

        let transport = mock_transport!(
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1011");
            "eth_getLogs" =>
                req => json!([{
                    "address": ["0x0000000000000000000000000000000000000001"],
                    "fromBlock": "0x4",
                    "limit": null,
                    "toBlock": "0x1005",
                    "topics": [[deposit_topic], null, null, null]
                }]),
                res => json!([]);
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1012");
            "eth_getLogs" =>
                req => json!([{
                    "address": ["0x0000000000000000000000000000000000000001"],
                    "fromBlock": "0x1006",
                    "limit": null,
                    "toBlock": "0x1006",
                    "topics": [[deposit_topic], null, null, null]
                }]),
                res => json!([]);
        );

        let log_stream = LogStream::new(LogStreamOptions {
            request_timeout: Duration::from_secs(1),
            poll_interval: Duration::from_secs(1),
            confirmations: 12,
            transport: transport.clone(),
            contract_address: "0000000000000000000000000000000000000001".into(),
            after: 3,
            filter: HomeBridge::default().events().deposit().create_filter(),
        });

        let mut event_loop = Core::new().unwrap();
        let log_ranges = event_loop.run(log_stream.take(2).collect()).unwrap();

        assert_eq!(
            log_ranges,
            vec![
                LogsInBlockRange {
                    from: 4,
                    to: 4101,
                    logs: vec![],
                },
                LogsInBlockRange {
                    from: 4102,
                    to: 4102,
                    logs: vec![],
                },
            ]
        );
        assert_eq!(transport.actual_requests(), transport.expected_requests());
    }

    #[test]
    fn test_log_stream_once_one_log() {
        let deposit_topic = HomeBridge::default()
            .events()
            .deposit()
            .create_filter()
            .topic0;

        let transport = mock_transport!(
            "eth_blockNumber" =>
                req => json!([]),
                res => json!("0x1011");
            "eth_getLogs" =>
                req => json!([{
                    "address": ["0x0000000000000000000000000000000000000001"],
                    "fromBlock": "0x4",
                    "limit": null,
                    "toBlock": "0x1005",
                    "topics": [[deposit_topic], null, null, null]
                }]),
                res => json!([{
                    "address": "0x0000000000000000000000000000000000000cc1",
                    "topics": [deposit_topic],
                    "data": "0x000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0",
                    "type": "",
                    "transactionHash": "0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364"
                }]);
        );

        let log_stream = LogStream::new(LogStreamOptions {
            request_timeout: Duration::from_secs(1),
            poll_interval: Duration::from_secs(1),
            confirmations: 12,
            transport: transport.clone(),
            contract_address: "0000000000000000000000000000000000000001".into(),
            after: 3,
            filter: HomeBridge::default().events().deposit().create_filter(),
        });

        let mut event_loop = Core::new().unwrap();
        let log_ranges = event_loop.run(log_stream.take(1).collect()).unwrap();

        assert_eq!(
            log_ranges,
            vec![
                LogsInBlockRange { from: 4, to: 4101, logs: vec![
                    Log {
                        address: "0x0000000000000000000000000000000000000cc1".into(),
                        topics: deposit_topic.into(),
                        data: Bytes("000000000000000000000000aff3454fce5edbc8cca8697c15331677e6ebcccc00000000000000000000000000000000000000000000000000000000000000f0".from_hex().unwrap()),
                        transaction_hash: Some("0x884edad9ce6fa2440d8a54cc123490eb96d2768479d49ff9c7366125a9424364".into()),
                        ..Default::default()
                    }
                ] },
            ]);
        assert_eq!(transport.actual_requests(), transport.expected_requests());
    }
}
