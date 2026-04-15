use std::future::Future;

/// Async loading of an environment from a data source.
///
/// Bridges the gap between async database access and the sync
/// methods on [`DescribeVar`](crate::DescribeVar). Implement
/// this for your environment type to enable the blanket
/// [`SourceVar`](crate::SourceVar) implementation and
/// [`Modeler::from_described`](crate::Modeler::from_described).
pub trait LoadEnv<Db>: Sized {
    /// Load the environment from the data source.
    fn load(db: &Db) -> impl Future<Output = Self>;
}

/// Identity impl: if you already have the environment, clone it.
impl<E: Clone + Send> LoadEnv<E> for E {
    async fn load(db: &E) -> E {
        db.clone()
    }
}
