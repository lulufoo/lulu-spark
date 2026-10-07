use crate::services::auth_login_landing::auth_login_landing_html;

use super::respond::respond_html;

pub(super) fn handle_auth_login_landing(request: tiny_http::Request) {
    respond_html(request, 200, auth_login_landing_html());
}
