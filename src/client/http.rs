use std::sync::Arc;
use warp::{Filter, Rejection, Reply};

use crate::client::handlers::process_connect_request;
use crate::client::network::SharedState;

pub fn create_connect_endpoint(
    state: Arc<SharedState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "connect")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::any().map(move || Arc::clone(&state)))
        .and(warp::body::bytes())
        .and_then(|state, request| process_connect_request(state, request))
        .with(warp::cors().allow_any_origin())
}
