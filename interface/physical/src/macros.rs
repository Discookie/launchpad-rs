#[macro_export]
macro_rules! send_or_err {
    ($control_request:expr, $msg:expr, $name:expr) => {
        if let Err(_) = $control_request.send($msg) {
            return Err(format!("{}: channel error", $name));
        }
    };
}

#[macro_export]
macro_rules! error_on_full {
    ($control_response:expr, $name:expr) => {
        if $control_response.is_full() {
            return Err(format!("{}: errored out", $name));
        }
    };
}