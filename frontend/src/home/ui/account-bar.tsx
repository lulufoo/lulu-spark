import { useEffect, useState } from 'react';

export type AccountBarUser = {
  user_id: string;
  email: string | null;
  display_name: string | null;
  avatar_url: string | null;
  provider: string | null;
};

export function AccountBar({
  user,
  onLogin,
  onLogout,
}: {
  user: AccountBarUser | null;
  onLogin: (provider: 'google' | 'github') => void;
  onLogout: () => void;
}) {
  const [open, setOpen] = useState(false);
  const signedIn = Boolean(user?.user_id);
  const label = signedInLabel(user);

  useEffect(() => {
    setOpen(false);
  }, [user?.user_id]);

  function close() {
    setOpen(false);
  }

  return (
    <div
      className="home-account-bar"
      data-role="account-bar"
      data-signed-in={signedIn ? 'true' : 'false'}
    >
      <button
        type="button"
        className="home-account-bar-trigger"
        data-role="account-bar-trigger"
        aria-haspopup="menu"
        aria-expanded={open ? 'true' : 'false'}
        aria-label={signedIn ? label : 'Sign in'}
        onClick={() => setOpen((prev) => !prev)}
      >
        {signedIn && user?.avatar_url ? (
          <img
            className="home-account-bar-avatar"
            data-role="account-bar-avatar"
            src={user.avatar_url}
            alt=""
          />
        ) : null}
        {signedIn ? (
          <span className="home-account-bar-name" data-role="account-bar-name">
            {label}
          </span>
        ) : (
          <span className="home-account-bar-label">Sign in</span>
        )}
      </button>
      {open ? (
        <>
          <div
            className="home-account-bar-backdrop"
            data-role="close-account-bar"
            onClick={close}
          />
          <div className="home-account-bar-menu" role="menu">
            {signedIn ? (
              <>
                <div className="home-account-bar-menu-name">{label}</div>
                <button
                  type="button"
                  role="menuitem"
                  data-role="account-logout"
                  onClick={() => {
                    callSafely(onLogout);
                    close();
                  }}
                >
                  Sign out
                </button>
              </>
            ) : (
              <>
                <button
                  type="button"
                  role="menuitem"
                  data-role="account-login-google"
                  onClick={() => {
                    callSafely(() => onLogin('google'));
                    close();
                  }}
                >
                  Google
                </button>
                <button
                  type="button"
                  role="menuitem"
                  data-role="account-login-github"
                  onClick={() => {
                    callSafely(() => onLogin('github'));
                    close();
                  }}
                >
                  GitHub
                </button>
              </>
            )}
          </div>
        </>
      ) : null}
    </div>
  );
}

function signedInLabel(user: AccountBarUser | null): string {
  if (!user?.user_id) return 'Sign in';
  return user.display_name?.trim() || user.email?.trim() || 'Signed in';
}

function callSafely(fn: () => void) {
  try {
    fn();
  } catch {
    /* keep the bar clickable */
  }
}
