//! # actix-web-middleware-redirect-https
//!
//! Provides a middleware for `actix-web` to redirect all `http` requests to `https`.

use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    http, Error, HttpResponse,
};
use futures::future::{ok, Either, Ready};
use std::task::{Context, Poll};

/// Middleware for `actix-web` which redirects all `http` requests to `https` with optional url
/// string replacements.
///
/// ## Usage
/// ```
/// use actix_web::{App, web, HttpResponse};
/// use actix_web_middleware_redirect_https::RedirectHTTPS;
///
/// App::new()
///     .wrap(RedirectHTTPS::default())
///     .route("/", web::get().to(|| HttpResponse::Ok()
///                                     .content_type("text/plain")
///                                     .body("Always HTTPS!")));
/// ```
#[derive(Default, Clone)]
pub struct RedirectHTTPS {
    disabled: bool,
    replacements: Vec<(String, String)>,
}

impl RedirectHTTPS {
    /// Creates a RedirectHTTPS middleware which also performs string replacement on the final url.
    /// This is useful when not running on the default web and ssl ports (80 and 443) since we will
    /// need to change the development web port in the hostname to the development ssl port.
    ///
    /// ## Usage
    /// ```
    /// use actix_web::{App, web, HttpResponse};
    /// use actix_web_middleware_redirect_https::RedirectHTTPS;
    ///
    /// App::new()
    ///     .wrap(RedirectHTTPS::with_replacements(&[(":8080".to_owned(), ":8443".to_owned())]))
    ///     .route("/", web::get().to(|| HttpResponse::Ok()
    ///                                     .content_type("text/plain")
    ///                                     .body("Always HTTPS on non-default ports!")));
    /// ```
    pub fn with_replacements(replacements: &[(String, String)]) -> Self {
        RedirectHTTPS {
            disabled: false,
            replacements: replacements.to_vec(),
        }
    }

    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.disabled = !enabled;
        self
    }
}

impl<S> Transform<S, ServiceRequest> for RedirectHTTPS
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = RedirectHTTPSService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RedirectHTTPSService {
            service,
            disabled: self.disabled,
            replacements: self.replacements.clone(),
        })
    }
}

pub struct RedirectHTTPSService<S> {
    service: S,
    disabled: bool,
    replacements: Vec<(String, String)>,
}

impl<S> Service<ServiceRequest> for RedirectHTTPSService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if req.connection_info().scheme() == "https" || !self.disabled {
            Either::Left(self.service.call(req))
        } else {
            let host = req.connection_info().host().to_owned();
            let uri = req.uri().to_owned();
            let mut url = format!("https://{}{}", host, uri);
            for (s1, s2) in self.replacements.iter() {
                url = url.replace(s1, s2);
            }
            Either::Right(ok(ServiceResponse::new(
                req.into_parts().0,
                HttpResponse::MovedPermanently()
                    .insert_header((http::header::LOCATION, url))
                    .finish()
          )))
        }
    }
}
