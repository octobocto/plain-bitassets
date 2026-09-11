use sneed::{DbError, EnvError, RwTxnError, env};

use crate::{
    archive, mempool, net, state,
    types::{AmountOverflowError, AmountUnderflowError, BlockHash, proto},
};

pub mod mainchain_task {
    use sneed::{EnvError, rwtxn::error as rwtxn};
    use thiserror::Error;

    use crate::{archive, types::proto};

    /// Error included in a response
    #[derive(Debug, Error)]
    pub enum RequestAncestorInfos {
        #[error("Archive error")]
        Archive(#[from] archive::Error),
        #[error("Database env error")]
        DbEnv(#[from] EnvError),
        #[error("Database write error")]
        DbWrite(#[from] rwtxn::Error),
        #[error("CUSF Mainchain proto error")]
        Mainchain(#[from] proto::Error),
    }

    #[derive(Debug, Error)]
    pub(in crate::node) enum SyncSideTipsToTip {
        #[error("Archive error")]
        Archive(#[from] archive::Error),
        #[error("Send event error")]
        SendEvent(#[from] futures::channel::mpsc::SendError),
        #[error("Database write error")]
        RwTxnCommit(#[from] rwtxn::Commit),
    }

    #[derive(Debug, Error)]
    pub(in crate::node) enum HandleBlockEvent {
        #[error("Archive error")]
        Archive(#[from] archive::Error),
        #[error("Database env error")]
        DbEnv(#[source] EnvError),
        #[error("Database write error")]
        DbWrite(#[source] rwtxn::Error),
        #[error("Send event error")]
        SendEvent(#[from] futures::channel::mpsc::SendError),
    }

    #[derive(Debug, Error)]
    pub(in crate::node) enum Error {
        #[error("Ancestor info for tip ({tip}) was unavailable")]
        AncestorInfoUnavailable { tip: bitcoin::BlockHash },
        #[error("Database env error")]
        DbEnv(#[source] EnvError),
        #[error(transparent)]
        HandleBlockEvent(#[from] HandleBlockEvent),
        #[error("CUSF Mainchain proto error")]
        Mainchain(#[from] proto::Error),
        #[error("Failed to fetch ancestor info for tip ({tip})")]
        RequestAncestorInfos {
            tip: bitcoin::BlockHash,
            source: Box<RequestAncestorInfos>,
        },
        #[error("Send event error")]
        SendEvent(#[source] futures::channel::mpsc::SendError),
        #[error("Send response error (oneshot)")]
        SendResponseOneshot,
        #[error("Failed to sync sidechain tips to mainchain tip ({tip})")]
        SyncSideTipsToTip {
            tip: bitcoin::BlockHash,
            source: Box<SyncSideTipsToTip>,
        },
    }
}

pub mod net_task {
    use std::sync::Arc;

    use futures::channel::{mpsc, oneshot};
    use sneed::{
        DbError, EnvError, RwTxnError, db, env::error as env_error,
        rwtxn::error as rwtxn_error,
    };
    use thiserror::Error;

    use crate::{
        archive, mempool, net, node::error::mainchain_task, state, types::proto,
    };

    #[derive(Debug, Error)]
    pub enum MainchainAncestors {
        #[error("Requested block was not available: {block_hash}")]
        BlockNotAvailable { block_hash: bitcoin::BlockHash },
        #[error(transparent)]
        MainchainTaskResponse(
            #[from] Arc<mainchain_task::RequestAncestorInfos>,
        ),
    }

    #[allow(clippy::duplicated_attributes)]
    #[derive(transitive::Transitive, Debug, Error)]
    #[transitive(
        from(db::error::IterInit, DbError),
        from(db::error::IterItem, DbError),
        from(env_error::WriteTxn, EnvError),
        from(rwtxn_error::Commit, RwTxnError)
    )]
    pub enum Error {
        #[error("archive error")]
        Archive(#[from] archive::Error),
        #[error("CUSF mainchain proto error")]
        CusfMainchain(#[from] proto::Error),
        #[error(transparent)]
        Db(#[from] DbError),
        #[error("Database env error")]
        DbEnv(#[from] EnvError),
        #[error("Database write error")]
        DbWrite(#[from] RwTxnError),
        #[error("Forward mainchain task request failed")]
        ForwardMainchainTaskRequest,
        #[error("mempool error")]
        MemPool(#[from] mempool::Error),
        #[error("Net error")]
        Net(#[from] Box<net::Error>),
        #[error("peer info stream closed")]
        PeerInfoRxClosed,
        #[error("Receive mainchain task response cancelled")]
        ReceiveMainchainTaskResponse,
        #[error("Receive reorg result cancelled (oneshot)")]
        ReceiveReorgResultOneshot(#[source] oneshot::Canceled),
        #[error("Send new tip ready failed")]
        SendNewTipReady(#[source] mpsc::SendError),
        #[error("Send reorg result error (oneshot)")]
        SendReorgResultOneshot,
        #[error("state error")]
        State(#[from] Box<state::Error>),
    }

    impl From<state::Error> for Error {
        fn from(err: state::Error) -> Self {
            Self::State(Box::new(err))
        }
    }
}

#[allow(clippy::duplicated_attributes)]
#[derive(thiserror::Error, transitive::Transitive, Debug)]
#[transitive(from(env::error::OpenEnv, EnvError))]
#[transitive(from(env::error::ReadTxn, EnvError))]
#[transitive(from(env::error::WriteTxn, EnvError))]
pub enum Error {
    #[error("address parse error")]
    AddrParse(#[from] std::net::AddrParseError),
    #[error(transparent)]
    AmountOverflow(#[from] AmountOverflowError),
    #[error(transparent)]
    AmountUnderflow(#[from] AmountUnderflowError),
    #[error("archive error")]
    Archive(#[from] archive::Error),
    #[error("CUSF mainchain proto error")]
    CusfMainchain(#[from] proto::Error),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error("Database env error")]
    DbEnv(#[from] EnvError),
    #[error("Database write error")]
    DbWrite(#[from] RwTxnError),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("error requesting mainchain ancestors")]
    MainchainAncestors(#[source] mainchain_task::RequestAncestorInfos),
    #[error("mempool error")]
    MemPool(#[from] mempool::Error),
    #[error("net error")]
    Net(#[source] Box<net::Error>),
    #[error("net task error")]
    NetTask(#[source] Box<net_task::Error>),
    #[error("No CUSF mainchain wallet client")]
    NoCusfMainchainWalletClient,
    #[error("block {block_hash} is not in the current chain")]
    NotInCurrentChain { block_hash: BlockHash },
    #[error("peer info stream closed")]
    PeerInfoRxClosed,
    #[error("Receive mainchain task response cancelled")]
    ReceiveMainchainTaskResponse,
    #[error("Send mainchain task request failed")]
    SendMainchainTaskRequest,
    #[error("state error")]
    State(#[source] Box<state::Error>),
    #[error("Utreexo error: {0}")]
    Utreexo(String),
    #[cfg(feature = "zmq")]
    #[error("ZMQ error")]
    Zmq(#[from] zeromq::ZmqError),
}

impl From<net::Error> for Error {
    fn from(err: net::Error) -> Self {
        Self::Net(Box::new(err))
    }
}

impl From<net_task::Error> for Error {
    fn from(err: net_task::Error) -> Self {
        Self::NetTask(Box::new(err))
    }
}

impl From<state::Error> for Error {
    fn from(err: state::Error) -> Self {
        Self::State(Box::new(err))
    }
}
