use http_02::HeaderMap as HttpHeaderMap;

/// Simple wrapper for grpc response with headers.
pub struct ResponseWrapper<T> {
    pub inner: T,
    pub headers: HttpHeaderMap,
}
