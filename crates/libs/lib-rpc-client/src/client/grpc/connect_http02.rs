#[cfg(not(any(feature = "__rustls")))]
compile_error!("Should at least enable one kind of TLS backend feature");
#[cfg(not(feature = "__tls"))]
compile_error!("TLS is a must for gRPC");

// ! Simple Connector Implementation
// ! Modified from `reqwest@ed9dbc7649f53cd18b5bdfe88173064c95bd6b78`
// ! Changes
// ! - Remove native-tls related code, using Rustls only

#[cfg(feature = "__tls")]
use http_02::header::HeaderValue;
use http_02::uri::{Authority, Scheme};
use http_02::Uri;
use hyper_014::client::connect::{Connected, Connection};
use hyper_014::service::Service;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use pin_project_lite::pin_project;
use std::future::Future;
use std::io::{self, IoSlice};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

#[cfg(feature = "__rustls")]
use self::rustls_tls_conn::RustlsTlsConn;
use super::proxy::{Proxy, ProxyScheme};
use crate::error::BoxError;

pub(crate) type HttpConnector = hyper_014::client::HttpConnector;

#[derive(Clone)]
/// Simple Connector implementation with Proxy support
///
/// # Known issues
///
/// - If ProxyScheme::HTTP && dst.scheme() == Some(&Scheme::HTTP), no authorization header is set
///   since such work should be done in `Client` part originally in `reqwest` implementation.
pub struct Connector {
    /// HTTP connector
    ///
    /// Presets:
    /// - SO_KEEPALIVE: 3600s (same with HTTP Keep-Alive timeout)
    /// - SO_NODELAY: true (disable Nagle's algorithm for lower latency)
    /// - Enforce HTTP: false
    http_connector: HttpConnector,

    /// Rustls config, wrapped in an `Arc` for cloning.
    tls_config: Arc<rustls::ClientConfig>,
    tls_config_proxy: Arc<rustls::ClientConfig>,

    /// Proxies, wrapped in an `Arc` for cloning.
    proxies: Arc<Vec<Proxy>>,

    /// Verbose logger
    verbose: verbose::Wrapper,

    /// Timeout for only the connect phase of a `Client`.
    /// Request timeout can be set in Client(not implemented)
    ///
    /// Defaults to 5s.
    connect_timeout: Option<Duration>,

    #[cfg(feature = "__tls")]
    /// Set the `SO_NODELAY` option.
    ///
    /// For notifing MaybeHttpsStream to set nodelay
    tcp_nodelay: bool,

    #[cfg(feature = "__tls")]
    /// user_agent for HTTP(S) Proxy
    user_agent: Option<HeaderValue>,
}

#[allow(dead_code)]
impl Connector {
    /// Create a new Connector with presets
    ///
    /// - SO_KEEPALIVE: 3600s (same with HTTP Keep-Alive timeout)
    /// - SO_NODELAY: true (disable Nagle's algorithm for lower latency)
    /// - Enforce HTTP: false
    /// - Verbose logger: false
    /// - Connect timeout: 5s
    pub fn new(proxies: Vec<Proxy>, user_agent: Option<HeaderValue>) -> Connector {
        const TCP_NODELAY: bool = true;

        let http_connector = {
            let mut http = hyper_014::client::connect::HttpConnector::new();
            http.set_keepalive(Some(Duration::from_secs(3600)));
            http.set_nodelay(TCP_NODELAY);
            http.enforce_http(false);
            http
        };

        let tls_config = super::tls::rustls_config(cfg!(test));

        // Clear ALPN for HTTP(s) Proxy
        // See: https://github.com/seanmonstar/reqwest/pull/466
        let mut tls_config_proxy = tls_config.clone();
        tls_config_proxy.alpn_protocols.clear();

        Self {
            http_connector,
            tls_config: Arc::new(tls_config),
            tls_config_proxy: Arc::new(tls_config_proxy),
            proxies: Arc::new(proxies),
            verbose: verbose::OFF,
            connect_timeout: Some(Duration::from_secs(5)),
            tcp_nodelay: true,
            user_agent,
        }
    }

    /// Set `SO_KEEPALIVE`.
    ///
    /// Default: 3600s
    pub(crate) fn set_tcp_keepalive(&mut self, keepalive: Option<Duration>) {
        self.http_connector.set_keepalive(keepalive);
    }

    /// Set `SO_NODELAY`.
    ///
    /// Default: true
    pub(crate) fn set_tcp_nodelay(&mut self, nodelay: bool) {
        self.tcp_nodelay = nodelay;
        self.http_connector.set_nodelay(nodelay);
    }

    /// Set the timeout for only the connect phase of a `Client`.
    ///
    /// Default: 5s
    pub(crate) fn set_connect_timeout(&mut self, timeout: Option<Duration>) {
        self.connect_timeout = timeout;
    }

    /// Set if verbose logging should be enabled.
    ///
    /// Default: false
    pub(crate) fn set_verbose(&mut self, enabled: bool) {
        self.verbose.0 = enabled;
    }

    /// Connect dst via given [`ProxyScheme`]
    async fn connect_via_proxy(
        self,
        dst: Uri,
        proxy_scheme: ProxyScheme,
    ) -> Result<Conn, BoxError> {
        tracing::debug!("Using proxy [{:?}] for [{:?}]", proxy_scheme, dst);

        let (proxy_dst, _auth) = match proxy_scheme {
            ProxyScheme::Http { host, auth } => (into_uri(Scheme::HTTP, host), auth),
            ProxyScheme::Https { host, auth } => (into_uri(Scheme::HTTPS, host), auth),
            #[cfg(feature = "socks")]
            ProxyScheme::Socks5 { .. } => return self.connect_socks(dst, proxy_scheme).await,
        };

        #[cfg(feature = "__tls")]
        let auth = _auth;

        // ! Warning
        // If ProxyScheme::HTTP && dst.scheme() == Some(&Scheme::HTTP), no authorization header is set
        // since such work should be done in `Client` part.
        if dst.scheme() == Some(&Scheme::HTTPS) {
            use rustls::ServerName;
            use tokio_rustls::TlsConnector as RustlsConnector;

            let host = dst.host().ok_or("no host in url")?.to_string();
            let port = dst.port().map(|r| r.as_u16()).unwrap_or(443);
            let server_name =
                ServerName::try_from(host.as_str()).map_err(|_| "Invalid Server Name")?;

            // tls_proxy is used for the HTTP/1.1 CONNECT request and alpn is cleared
            let proxy_conn = hyper_rustls::HttpsConnector::from((
                self.http_connector.clone(),
                self.tls_config_proxy.clone(),
            ))
            .call(proxy_dst)
            .await?;

            tracing::debug!("tunneling HTTPS over http(s) proxy");

            let tunneled = tunnel(proxy_conn, host, port, self.user_agent.clone(), auth).await?;

            let io = RustlsConnector::from(self.tls_config.clone())
                .connect(server_name, tunneled)
                .await?;

            return Ok(Conn {
                inner: self.verbose.wrap(RustlsTlsConn { inner: io }),
                is_proxy: false,
            });
        }

        self.connect_with_maybe_proxy(proxy_dst, true).await
    }

    #[cfg(feature = "socks")]
    async fn connect_socks(&self, dst: Uri, proxy: ProxyScheme) -> Result<Conn, BoxError> {
        let dns = match proxy {
            ProxyScheme::Socks5 {
                remote_dns: false, ..
            } => socks::DnsResolve::Local,
            ProxyScheme::Socks5 {
                remote_dns: true, ..
            } => socks::DnsResolve::Proxy,
            ProxyScheme::Http { .. } | ProxyScheme::Https { .. } => {
                unreachable!("connect_socks is only called for socks proxies");
            }
        };

        let socks_conn = socks::connect(proxy, &dst, dns).await?;

        let conn = if dst.scheme() == Some(&Scheme::HTTPS) {
            use tokio_rustls::TlsConnector as RustlsConnector;

            let server_name = rustls::ServerName::try_from(dst.host().ok_or("no host in url")?)
                .map_err(|_| "Invalid Server Name")?;
            let tls_config = self.tls_config.clone();
            let tls_stream = RustlsConnector::from(tls_config)
                .connect(server_name, socks_conn)
                .await?;

            Conn {
                inner: self.verbose.wrap(RustlsTlsConn { inner: tls_stream }),
                is_proxy: false,
            }
        } else {
            Conn {
                inner: self.verbose.wrap(socks_conn),
                is_proxy: false,
            }
        };

        Ok(conn)
    }

    async fn connect_with_maybe_proxy(self, dst: Uri, is_proxy: bool) -> Result<Conn, BoxError> {
        let mut http = self.http_connector.clone();

        // Disable Nagle's algorithm for TLS handshake
        //
        // https://www.openssl.org/docs/man1.1.1/man3/SSL_connect.html#NOTES
        if !self.tcp_nodelay && (dst.scheme() == Some(&Scheme::HTTPS)) {
            http.set_nodelay(true);
        }

        let mut http = hyper_rustls::HttpsConnector::from((http, self.tls_config.clone()));
        let io = http.call(dst).await?;

        if let hyper_rustls::MaybeHttpsStream::Https(stream) = io {
            if !self.tcp_nodelay {
                let (io, _) = stream.get_ref();
                io.set_nodelay(false)?;
            }
            Ok(Conn {
                inner: self.verbose.wrap(RustlsTlsConn { inner: stream }),
                is_proxy,
            })
        } else {
            Ok(Conn {
                inner: self.verbose.wrap(io),
                is_proxy,
            })
        }
    }
}

fn into_uri(scheme: Scheme, host: Authority) -> Uri {
    // TODO: Should the `http` crate get `From<(Scheme, Authority)> for Uri`?
    http_02::Uri::builder()
        .scheme(scheme)
        .authority(host)
        .path_and_query(http_02::uri::PathAndQuery::from_static("/"))
        .build()
        .expect("scheme and authority is valid Uri")
}

async fn with_timeout<T, F>(f: F, timeout: Option<Duration>) -> Result<T, BoxError>
where
    F: Future<Output = Result<T, BoxError>>,
{
    if let Some(to) = timeout {
        match tokio::time::timeout(to, f).await {
            Err(_elapsed) => Err("Operation timed out".into()),
            Ok(Ok(try_res)) => Ok(try_res),
            Ok(Err(e)) => Err(e),
        }
    } else {
        f.await
    }
}

impl Service<Uri> for Connector {
    type Response = Conn;
    type Error = BoxError;
    type Future = Connecting;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        tracing::debug!("starting new connection: {:?}", dst);
        let timeout = self.connect_timeout;

        let proxy_scheme = self.proxies.iter().find(|p| p.is_available()).map_or_else(
            || {
                self.proxies.first().map(|p| {
                    tracing::warn!(
                        "No available proxy found in given proxies, use the first one: [{:?}].",
                        p.scheme()
                    );
                    p.scheme_owned()
                })
            },
            |p| Some(p.scheme_owned()),
        );

        if let Some(proxy_scheme) = proxy_scheme {
            return Box::pin(with_timeout(
                self.clone().connect_via_proxy(dst, proxy_scheme),
                timeout,
            ));
        }

        Box::pin(with_timeout(
            self.clone().connect_with_maybe_proxy(dst, false),
            timeout,
        ))
    }
}

pub(crate) trait AsyncConn:
    AsyncRead + AsyncWrite + Connection + Send + Sync + Unpin + 'static
{
}

impl<T: AsyncRead + AsyncWrite + Connection + Send + Sync + Unpin + 'static> AsyncConn for T {}

type BoxConn = Box<dyn AsyncConn>;

pin_project! {
    /// Note: the `is_proxy` member means *is plain text HTTP proxy*.
    /// This tells hyper_014 whether the URI should be written in
    /// * origin-form (`GET /just/a/path HTTP/1.1`), when `is_proxy == false`, or
    /// * absolute-form (`GET http://foo.bar/and/a/path HTTP/1.1`), otherwise.
    pub struct Conn {
        #[pin]
        inner: BoxConn,
        is_proxy: bool,
    }
}

impl Connection for Conn {
    fn connected(&self) -> Connected {
        self.inner.connected().proxy(self.is_proxy)
    }
}

impl AsyncRead for Conn {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.project();
        AsyncRead::poll_read(this.inner, cx, buf)
    }
}

impl AsyncWrite for Conn {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.project();
        AsyncWrite::poll_write(this.inner, cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.project();
        AsyncWrite::poll_write_vectored(this.inner, cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        AsyncWrite::poll_flush(this.inner, cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        AsyncWrite::poll_shutdown(this.inner, cx)
    }
}

pub(crate) type Connecting = Pin<Box<dyn Future<Output = Result<Conn, BoxError>> + Send>>;

#[cfg(feature = "__tls")]
async fn tunnel<T>(
    mut conn: T,
    host: String,
    port: u16,
    user_agent: Option<HeaderValue>,
    auth: Option<HeaderValue>,
) -> Result<T, BoxError>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = format!(
        "\
         CONNECT {0}:{1} HTTP/1.1\r\n\
         Host: {0}:{1}\r\n\
         ",
        host, port
    )
    .into_bytes();

    // user-agent
    if let Some(user_agent) = user_agent {
        buf.extend_from_slice(b"User-Agent: ");
        buf.extend_from_slice(user_agent.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    // proxy-authorization
    if let Some(value) = auth {
        tracing::debug!("tunnel to {}:{} using basic auth", host, port);
        buf.extend_from_slice(b"Proxy-Authorization: ");
        buf.extend_from_slice(value.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    // headers end
    buf.extend_from_slice(b"\r\n");

    conn.write_all(&buf).await?;

    let mut buf = [0; 8192];
    let mut pos = 0;

    loop {
        let n = conn.read(&mut buf[pos..]).await?;

        if n == 0 {
            return Err(tunnel_eof());
        }
        pos += n;

        let recvd = &buf[..pos];
        if recvd.starts_with(b"HTTP/1.1 200") || recvd.starts_with(b"HTTP/1.0 200") {
            if recvd.ends_with(b"\r\n\r\n") {
                return Ok(conn);
            }
            if pos == buf.len() {
                return Err("proxy headers too long for tunnel".into());
            }
        // else read more
        } else if recvd.starts_with(b"HTTP/1.1 407") {
            return Err("proxy authentication required".into());
        } else {
            return Err("unsuccessful tunnel".into());
        }
    }
}

#[cfg(feature = "__tls")]
fn tunnel_eof() -> BoxError {
    "unexpected eof while tunneling".into()
}

#[cfg(feature = "__rustls")]
mod rustls_tls_conn {
    use hyper_014::client::connect::{Connected, Connection};
    use pin_project_lite::pin_project;
    use std::{
        io::{self, IoSlice},
        pin::Pin,
        task::{Context, Poll},
    };
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio_rustls::client::TlsStream;

    pin_project! {
        pub(super) struct RustlsTlsConn<T> {
            #[pin] pub(super) inner: TlsStream<T>,
        }
    }

    impl<T: Connection + AsyncRead + AsyncWrite + Unpin> Connection for RustlsTlsConn<T> {
        fn connected(&self) -> Connected {
            if self.inner.get_ref().1.alpn_protocol() == Some(b"h2") {
                self.inner.get_ref().0.connected().negotiated_h2()
            } else {
                self.inner.get_ref().0.connected()
            }
        }
    }

    impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for RustlsTlsConn<T> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<tokio::io::Result<()>> {
            let this = self.project();
            AsyncRead::poll_read(this.inner, cx, buf)
        }
    }

    impl<T: AsyncRead + AsyncWrite + Unpin> AsyncWrite for RustlsTlsConn<T> {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<Result<usize, tokio::io::Error>> {
            let this = self.project();
            AsyncWrite::poll_write(this.inner, cx, buf)
        }

        fn poll_write_vectored(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &[IoSlice<'_>],
        ) -> Poll<Result<usize, io::Error>> {
            let this = self.project();
            AsyncWrite::poll_write_vectored(this.inner, cx, bufs)
        }

        fn is_write_vectored(&self) -> bool {
            self.inner.is_write_vectored()
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            cx: &mut Context,
        ) -> Poll<Result<(), tokio::io::Error>> {
            let this = self.project();
            AsyncWrite::poll_flush(this.inner, cx)
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            cx: &mut Context,
        ) -> Poll<Result<(), tokio::io::Error>> {
            let this = self.project();
            AsyncWrite::poll_shutdown(this.inner, cx)
        }
    }
}

#[cfg(feature = "socks")]
mod socks {
    use std::io;
    use std::net::ToSocketAddrs;

    use http_02::Uri;
    use tokio::net::TcpStream;
    use tokio_socks::tcp::Socks5Stream;

    use super::ProxyScheme;
    use super::{BoxError, Scheme};

    pub(super) enum DnsResolve {
        Local,
        Proxy,
    }

    pub(super) async fn connect(
        proxy: ProxyScheme,
        dst: &Uri,
        dns: DnsResolve,
    ) -> Result<TcpStream, BoxError> {
        let https = dst.scheme() == Some(&Scheme::HTTPS);
        let original_host = dst
            .host()
            .ok_or(io::Error::new(io::ErrorKind::Other, "no host in url"))?;
        let mut host = original_host.to_owned();
        let port = match dst.port() {
            Some(p) => p.as_u16(),
            None if https => 443u16,
            _ => 80u16,
        };

        if let DnsResolve::Local = dns {
            let maybe_new_target = (host.as_str(), port).to_socket_addrs()?.next();
            if let Some(new_target) = maybe_new_target {
                host = new_target.ip().to_string();
            }
        }

        let (socket_addr, auth) = match proxy {
            ProxyScheme::Socks5 { addr, auth, .. } => (addr, auth),
            _ => unreachable!(),
        };

        // Get a Tokio TcpStream
        let stream = if let Some((username, password)) = auth {
            Socks5Stream::connect_with_password(
                socket_addr,
                (host.as_str(), port),
                &username,
                &password,
            )
            .await
            .map_err(|e| format!("socks connect error: {}", e))?
        } else {
            Socks5Stream::connect(socket_addr, (host.as_str(), port))
                .await
                .map_err(|e| format!("socks connect error: {}", e))?
        };

        Ok(stream.into_inner())
    }
}

mod verbose {
    use hyper_014::client::connect::{Connected, Connection};
    use std::cmp::min;
    use std::fmt;
    use std::io::{self, IoSlice};
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

    pub(super) const OFF: Wrapper = Wrapper(false);

    #[derive(Clone, Copy)]
    pub(super) struct Wrapper(pub(super) bool);

    impl Wrapper {
        pub(super) fn wrap<T: super::AsyncConn>(&self, conn: T) -> super::BoxConn {
            if self.0 && log::log_enabled!(log::Level::Trace) {
                Box::new(Verbose {
                    // truncate is fine
                    id: crate::utils::fast_random() as u32,
                    inner: conn,
                })
            } else {
                Box::new(conn)
            }
        }
    }

    struct Verbose<T> {
        id: u32,
        inner: T,
    }

    impl<T: Connection + AsyncRead + AsyncWrite + Unpin> Connection for Verbose<T> {
        fn connected(&self) -> Connected {
            self.inner.connected()
        }
    }

    impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for Verbose<T> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            match Pin::new(&mut self.inner).poll_read(cx, buf) {
                Poll::Ready(Ok(())) => {
                    tracing::trace!("{:08x} read: {:?}", self.id, Escape(buf.filled()));
                    Poll::Ready(Ok(()))
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    impl<T: AsyncRead + AsyncWrite + Unpin> AsyncWrite for Verbose<T> {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<Result<usize, std::io::Error>> {
            match Pin::new(&mut self.inner).poll_write(cx, buf) {
                Poll::Ready(Ok(n)) => {
                    tracing::trace!("{:08x} write: {:?}", self.id, Escape(&buf[..n]));
                    Poll::Ready(Ok(n))
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }

        fn poll_write_vectored(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &[IoSlice<'_>],
        ) -> Poll<Result<usize, io::Error>> {
            match Pin::new(&mut self.inner).poll_write_vectored(cx, bufs) {
                Poll::Ready(Ok(nwritten)) => {
                    tracing::trace!(
                        "{:08x} write (vectored): {:?}",
                        self.id,
                        Vectored { bufs, nwritten }
                    );
                    Poll::Ready(Ok(nwritten))
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }

        fn is_write_vectored(&self) -> bool {
            self.inner.is_write_vectored()
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
        ) -> Poll<Result<(), std::io::Error>> {
            Pin::new(&mut self.inner).poll_flush(cx)
        }

        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
        ) -> Poll<Result<(), std::io::Error>> {
            Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }

    struct Escape<'a>(&'a [u8]);

    impl fmt::Debug for Escape<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "b\"")?;
            for &c in self.0 {
                // https://doc.rust-lang.org/reference.html#byte-escapes
                if c == b'\n' {
                    write!(f, "\\n")?;
                } else if c == b'\r' {
                    write!(f, "\\r")?;
                } else if c == b'\t' {
                    write!(f, "\\t")?;
                } else if c == b'\\' || c == b'"' {
                    write!(f, "\\{}", c as char)?;
                } else if c == b'\0' {
                    write!(f, "\\0")?;
                // ASCII printable
                } else if c >= 0x20 && c < 0x7f {
                    write!(f, "{}", c as char)?;
                } else {
                    write!(f, "\\x{:02x}", c)?;
                }
            }
            write!(f, "\"")?;
            Ok(())
        }
    }

    struct Vectored<'a, 'b> {
        bufs: &'a [IoSlice<'b>],
        nwritten: usize,
    }

    impl fmt::Debug for Vectored<'_, '_> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut left = self.nwritten;
            for buf in self.bufs.iter() {
                if left == 0 {
                    break;
                }
                let n = min(left, buf.len());
                Escape(&buf[..n]).fmt(f)?;
                left -= n;
            }
            Ok(())
        }
    }
}

#[cfg(feature = "__tls")]
#[cfg(test)]
mod tests {
    use super::tunnel;
    use super::super::proxy;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use tokio::net::TcpStream;
    use tokio::runtime;

    static TUNNEL_UA: &str = "tunnel-test/x.y";
    static TUNNEL_OK: &[u8] = b"\
        HTTP/1.1 200 OK\r\n\
        \r\n\
    ";

    macro_rules! mock_tunnel {
        () => {{
            mock_tunnel!(TUNNEL_OK)
        }};
        ($write:expr) => {{
            mock_tunnel!($write, "")
        }};
        ($write:expr, $auth:expr) => {{
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let connect_expected = format!(
                "\
                 CONNECT {0}:{1} HTTP/1.1\r\n\
                 Host: {0}:{1}\r\n\
                 User-Agent: {2}\r\n\
                 {3}\
                 \r\n\
                 ",
                addr.ip(),
                addr.port(),
                TUNNEL_UA,
                $auth
            )
            .into_bytes();

            thread::spawn(move || {
                let (mut sock, _) = listener.accept().unwrap();
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).unwrap();
                assert_eq!(&buf[..n], &connect_expected[..]);

                sock.write_all($write).unwrap();
            });
            addr
        }};
    }

    fn ua() -> Option<http_02::header::HeaderValue> {
        Some(http_02::header::HeaderValue::from_static(TUNNEL_UA))
    }

    #[test]
    fn test_tunnel() {
        let addr = mock_tunnel!();

        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("new rt");
        let f = async move {
            let tcp = TcpStream::connect(&addr).await?;
            let host = addr.ip().to_string();
            let port = addr.port();
            tunnel(tcp, host, port, ua(), None).await
        };

        rt.block_on(f).unwrap();
    }

    #[test]
    fn test_tunnel_eof() {
        let addr = mock_tunnel!(b"HTTP/1.1 200 OK");

        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("new rt");
        let f = async move {
            let tcp = TcpStream::connect(&addr).await?;
            let host = addr.ip().to_string();
            let port = addr.port();
            tunnel(tcp, host, port, ua(), None).await
        };

        rt.block_on(f).unwrap_err();
    }

    #[test]
    fn test_tunnel_non_http_response() {
        let addr = mock_tunnel!(b"foo bar baz hallo");

        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("new rt");
        let f = async move {
            let tcp = TcpStream::connect(&addr).await?;
            let host = addr.ip().to_string();
            let port = addr.port();
            tunnel(tcp, host, port, ua(), None).await
        };

        rt.block_on(f).unwrap_err();
    }

    #[test]
    fn test_tunnel_proxy_unauthorized() {
        let addr = mock_tunnel!(
            b"\
            HTTP/1.1 407 Proxy Authentication Required\r\n\
            Proxy-Authenticate: Basic realm=\"nope\"\r\n\
            \r\n\
        "
        );

        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("new rt");
        let f = async move {
            let tcp = TcpStream::connect(&addr).await?;
            let host = addr.ip().to_string();
            let port = addr.port();
            tunnel(tcp, host, port, ua(), None).await
        };

        let error = rt.block_on(f).unwrap_err();
        assert_eq!(error.to_string(), "proxy authentication required");
    }

    #[test]
    fn test_tunnel_basic_auth() {
        let addr = mock_tunnel!(
            TUNNEL_OK,
            "Proxy-Authorization: Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==\r\n"
        );

        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("new rt");
        let f = async move {
            let tcp = TcpStream::connect(&addr).await?;
            let host = addr.ip().to_string();
            let port = addr.port();
            tunnel(
                tcp,
                host,
                port,
                ua(),
                Some(proxy::encode_basic_auth(Some((
                    "Aladdin".into(),
                    "open sesame".into(),
                ))))
                .unwrap(),
            )
            .await
        };

        rt.block_on(f).unwrap();
    }
}
