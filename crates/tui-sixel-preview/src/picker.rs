use ratatui_image::picker::Picker;

use crate::logging::write_terminal_protocol_log;

pub fn build_picker() -> Picker {
    match Picker::from_query_stdio() {
        Ok(picker) => {
            let _ = write_terminal_protocol_log(&picker, "query_stdio", None);
            picker
        }
        Err(error) => {
            let picker = Picker::halfblocks();
            let error_message = error.to_string();
            let _ =
                write_terminal_protocol_log(&picker, "halfblocks_fallback", Some(&error_message));
            picker
        }
    }
}
