#[allow(dead_code)]
pub struct AdapticsError {
    context: Option<String>,
    wrapped: Option<Box<dyn std::error::Error + Send + Sync>>,
    backtrace: std::backtrace::Backtrace,
}
impl AdapticsError {
    #[must_use]
    pub fn new(context: &str) -> AdapticsError {
        AdapticsError{ context: Some(context.to_string()), wrapped: None, backtrace: std::backtrace::Backtrace::capture() }
    }
    #[must_use]
    pub fn wrap(wrapped: Box<dyn std::error::Error + Send + Sync>) -> AdapticsError {
        let backtrace = std::backtrace::Backtrace::capture();
        AdapticsError{ context: None, wrapped: Some(wrapped), backtrace }
    }
}
impl std::fmt::Debug for AdapticsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,"{{ context: {:?}, wrapped: {:?} backtrace: {:#?} }}", self.context, self.wrapped, self.backtrace)
    }
}
impl std::fmt::Display for AdapticsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match (&self.wrapped, &self.context) {
            (Some(wrapped), Some(context)) => write!(f,"{context}: {wrapped}"),
            (Some(wrapped), None) => write!(f,"{wrapped}"),
            (None, Some(context)) => write!(f,"{context}"),
            (None, None) => write!(f,"<unknown error>")
        }
    }
}
impl<T> From<T> for AdapticsError
where T: std::error::Error + Send + Sync + 'static {
    fn from(error: T) -> Self {
        AdapticsError::wrap(Box::new(error))
    }
}
// impl From<&str> for AdapticsError {
//     fn from(context: &str) -> Self {
//         AdapticsError::new(context.as_ref())
//     }
// }

mod tlw_std_error { // need to use two types because cant impl From<std::Error> for TLWError if there is an impl std::Error for TLWError
    type AdapticsError = super::AdapticsError;

    #[allow(dead_code)]
    struct StdAdapticsError(AdapticsError);
    impl std::fmt::Display for StdAdapticsError { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f,"{}",self.0) } }
    impl std::fmt::Debug for StdAdapticsError { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f,"{:?}",self.0) } }
    impl std::error::Error for StdAdapticsError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.0.wrapped.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
        }
    }


    impl From<AdapticsError> for Box<dyn std::error::Error + Send + Sync + 'static> {
        fn from(error: AdapticsError) -> Self {
            Box::new(StdAdapticsError(error))
        }
    }
    impl From<AdapticsError> for Box<dyn std::error::Error + Send + 'static> {
        fn from(error: AdapticsError) -> Self {
            Box::<dyn std::error::Error + Send + Sync>::from(error)
        }
    }
    impl From<AdapticsError> for Box<dyn std::error::Error + 'static> {
        fn from(error: AdapticsError) -> Self {
            Box::<dyn std::error::Error + Send + Sync>::from(error)
        }
    }
}
