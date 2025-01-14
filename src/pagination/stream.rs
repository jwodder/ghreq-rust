use super::{PageRequest, PageResponse, PaginationInfo, PaginationRequest, PaginationState};
use crate::{
    client::tokio::{AsyncBackend, AsyncClient},
    errors::Error,
    Endpoint,
};
use futures_util::{future::BoxFuture, stream::FusedStream, FutureExt, Stream};
use pin_project_lite::pin_project;
use serde::de::DeserializeOwned;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct PaginationStream<B: AsyncBackend, R: PaginationRequest> {
        client: AsyncClient<B>,
        req: R,
        inner: InnerState<R::Item, B::Error>,
        info: Option<PaginationInfo>,
        state: PaginationState,
    }
}

impl<B: AsyncBackend, R: PaginationRequest> PaginationStream<B, R> {
    pub fn new(client: AsyncClient<B>, req: R) -> Self {
        let next_url = Some(req.endpoint());
        PaginationStream {
            client,
            req,
            inner: InnerState::Yielding {
                items: Vec::new().into_iter(),
                next_url,
            },
            info: None,
            state: PaginationState::NotStarted,
        }
    }

    pub fn info(&self) -> Option<PaginationInfo> {
        self.info
    }

    pub fn state(&self) -> PaginationState {
        self.state
    }
}

impl<B, R> Stream for PaginationStream<B, R>
where
    B: AsyncBackend + Clone + Send + Sync + 'static,
    R: PaginationRequest<Item: DeserializeOwned + Send + 'static>,
{
    type Item = Result<R::Item, Error<B::Error>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        loop {
            match this.inner {
                InnerState::Requesting(ref mut fut) => match ready!(fut.as_mut().poll(cx)) {
                    Ok(page_resp) => {
                        *this.state = PaginationState::Paging;
                        *this.inner = InnerState::Yielding {
                            items: page_resp.items.into_iter(),
                            next_url: page_resp.next_url.map(Into::into),
                        };
                        *this.info = Some(page_resp.info);
                    }
                    Err(e) => {
                        *this.state = PaginationState::Ended;
                        *this.inner = InnerState::Done;
                        *this.info = None;
                        return Some(Err(e)).into();
                    }
                },
                InnerState::Yielding {
                    ref mut items,
                    ref mut next_url,
                } => {
                    if let Some(value) = items.next() {
                        return Some(Ok(value)).into();
                    } else if let Some(url) = next_url.take() {
                        let mut req = PageRequest::new(url.clone())
                            .with_headers(this.req.headers())
                            .with_timeout(this.req.timeout());
                        if *this.state == PaginationState::NotStarted {
                            req = req.with_params(this.req.params());
                        }
                        let client = this.client.clone();
                        *this.inner = InnerState::Requesting(
                            async move { client.clone().request(req).await }.boxed(),
                        );
                    } else {
                        *this.state = PaginationState::Ended;
                        *this.inner = InnerState::Done;
                        *this.info = None;
                    }
                }
                InnerState::Done => return None.into(),
            }
        }
    }
}

impl<B, R> FusedStream for PaginationStream<B, R>
where
    B: AsyncBackend + Clone + Send + Sync + 'static,
    R: PaginationRequest<Item: DeserializeOwned + Send + 'static>,
{
    fn is_terminated(&self) -> bool {
        self.state == PaginationState::Ended
    }
}

enum InnerState<T, BE> {
    Requesting(BoxFuture<'static, Result<PageResponse<T>, Error<BE>>>),
    Yielding {
        items: std::vec::IntoIter<T>,
        next_url: Option<Endpoint>,
    },
    Done,
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[test]
    fn stream_next_is_send() {
        #[allow(dead_code)]
        fn require_send<T: Send>(_t: T) {}

        #[allow(dead_code)]
        fn check<B, R>(stream: PaginationStream<B, R>)
        where
            B: AsyncBackend + Clone + Send + Sync + 'static,
            R: PaginationRequest<Item: DeserializeOwned + Send + 'static> + Send,
        {
            tokio::pin!(stream);
            require_send(stream.next());
        }
    }
}
