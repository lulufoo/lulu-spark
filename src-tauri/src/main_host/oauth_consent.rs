use crate::services::oauth_consent::oauth_consent_html;

use super::respond::respond_html;

pub(super) fn handle_oauth_consent(request: tiny_http::Request) {
    respond_html(request, 200, oauth_consent_html());
}
