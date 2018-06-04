use futures::{Async, Future, Poll, Stream};
use futures::future::{join_all, FromErr, Join, JoinAll};
use tokio_timer::Timeout;
use web3::Transport;
use web3::types::{Address, Bytes, H256, Log, U256, TransactionReceipt};
use web3;
use ethabi::{self, RawLog};
use log_stream::LogStream;
use contracts;
use contracts::home::HomeBridge;
use contracts::foreign::ForeignBridge;
use error::{self, ResultExt};
use message_to_main::MessageToMain;
use signature::Signature;
use web3::helpers::CallResult;
use relay_stream::LogToFuture;
use side_contract::SideContract;
use main_contract::MainContract;
use web3::api::Namespace;
use helpers::{AsyncCall, AsyncTransaction};
use helpers;

/// state of the state machine that is the future responsible for
/// the SideToMain relay
enum State<T: Transport> {
    AwaitMessage(AsyncCall<T, contracts::foreign::MessageWithInput>),
    /// authority is not responsible for relaying this. noop
    NotResponsible,
    AwaitIsRelayed {
        future: AsyncCall<T, contracts::home::WithdrawsWithInput>,
        message: MessageToMain,
    },
    AwaitSignatures {
        future: JoinAll<Vec<AsyncCall<T, contracts::foreign::SignatureWithInput>>>,
        message: MessageToMain,
    },
    AwaitTxSent(AsyncTransaction<T>),
    // AwaitTxReceipt(Timeout<FromErr<CallResult<Option<TransactionReceipt>, T::Out>, error::Error>>),
}

pub struct SideToMainSignatures<T: Transport> {
    side_tx_hash: H256,
    main: MainContract<T>,
    side: SideContract<T>,
    state: State<T>,
}

impl<T: Transport> SideToMainSignatures<T> {
    pub fn new(raw_log: &Log, main: MainContract<T>, side: SideContract<T>) -> Self {
        let side_tx_hash = raw_log.transaction_hash
            .expect("`log` must be mined and contain `transaction_hash`. q.e.d.");

        let log = helpers::parse_log(
            &ForeignBridge::default().events().collected_signatures(),
            raw_log
        ).expect("`Log` must be a from a `CollectedSignatures` event. q.e.d.");

        let state = if log.authority_responsible_for_relay != main.authority_address {
            info!(
                "{:?} - this bridge node is not responsible for relaying transaction to main",
                side_tx_hash
            );
            // this bridge node is not responsible for relaying this transaction.
            // someone else will relay this transaction to home.
            State::NotResponsible
        } else {
            info!(
                "{:?} - step 1/3 - about to fetch message",
                side_tx_hash,
            );
            State::AwaitMessage(side.call(
                ForeignBridge::default().functions().message(log.message_hash)
            ))
        };

        Self {
            side_tx_hash,
            main,
            side,
            state,
        }
    }
}

impl<T: Transport> Future for SideToMainSignatures<T> {
    type Item = Option<TransactionReceipt>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                State::NotResponsible => {
                    return Ok(Async::Ready(None));
                }
                State::AwaitMessage(ref mut future) => {
                    let message = try_ready!(future.poll().chain_err(|| "SubmitSignature: fetching message failed"));
                    State::AwaitIsRelayed {
                        future: self.main.call(
                            HomeBridge::default().functions().withdraws(message.side_tx_hash)
                        ),
                        message
                    }
                }
                State::AwaitIsRelayed { ref mut future, message } => {
                    let is_relayed = try_ready!(future.poll().chain_err(|| "SubmitSignature: fetching message failed"));

                    if is_relayed {
                        return Ok(Async::Ready(None));
                    }

                    State::AwaitSignatures {
                        future: self.side.get_signatures(message.keccak256()),
                        message
                    }
                }
                State::AwaitSignatures { ref mut future, message }  => {
                    let signatures = try_ready!(future.poll().chain_err(|| "WithdrawRelay: fetching message and signatures failed"));
                    info!("{:?} - step 2/3 - message and {} signatures received. about to send transaction", self.side_tx_hash, signatures.len());
                    State::AwaitTxSent(self.main.relay_side_to_main(&message, &signatures))
                }
                State::AwaitTxSent(ref mut future) => {
                    let main_tx_hash = try_ready!(
                        future
                            .poll()
                            .chain_err(|| "WithdrawRelay: sending transaction failed")
                    );
                    info!(
                        "{:?} - step 3/3 - DONE - transaction sent {:?}",
                        self.side_tx_hash, main_tx_hash
                    );
                    State::AwaitTxReceipt(web3::api::Eth::new(self.side.transport)
                        .transaction_receipt(main_tx_hash))
                }
                State::AwaitTxReceipt(ref mut future) => {
                    let receipt = try_ready!(future.poll().chain_err(|| "WithdrawRelay: sending transaction failed")).unwrap();
                    return Ok(Async::Ready(Some(receipt)));
                }
            };
            self.state = next_state;
        }
    }
}

/// options for relays from side to main
pub struct LogToSideToMainSignatures<T> {
    pub main: MainContract<T>,
    pub side: SideContract<T>,
}

/// from the options and a log a relay future can be made
impl<T: Transport> LogToFuture for LogToSideToMainSignatures<T> {
    type Future = SideToMainSignatures<T>;

    fn log_to_future(&self, log: &Log) -> Self::Future {
        SideToMainSignatures::new(log, self.main.clone(), self.side.clone())
    }
}
