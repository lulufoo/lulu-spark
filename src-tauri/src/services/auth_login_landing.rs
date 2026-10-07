//! Host-produced auth-login landing HTML. No HTTP bind.

pub fn auth_login_landing_html() -> String {
    r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body>
<script>
location.assign("spark://auth-login/callback" + location.search + location.hash);
</script>
</body>
</html>
"#
    .to_string()
}

#[cfg(test)]
#[path = "../unit-tests/services/auth_login_landing.rs"]
mod tests;
