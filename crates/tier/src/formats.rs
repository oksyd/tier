mod email;
mod hostname;
mod shared;
mod url;

pub(crate) use self::email::is_valid_email;
pub(crate) use self::hostname::is_valid_hostname;
pub(crate) use self::url::is_valid_url;
