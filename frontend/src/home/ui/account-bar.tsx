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
  isDebug = false,
}: {
  user: AccountBarUser | null;
  onLogin: (provider: 'google' | 'github') => void;
  onLogout: () => void;
  isDebug?: boolean;
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

  function chooseProvider(provider: 'google' | 'github') {
    callSafely(() => onLogin(provider));
    close();
  }

  return (
    <div
      className="home-account-bar"
      data-role="account-bar"
      data-signed-in={signedIn ? 'true' : 'false'}
      data-debug={isDebug ? 'true' : 'false'}
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
        {signedIn && user ? <SignedInAvatar user={user} /> : <GuestAvatar />}
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
                  onClick={() => chooseProvider('google')}
                >
                  Google
                </button>
                <button
                  type="button"
                  role="menuitem"
                  data-role="account-login-github"
                  onClick={() => chooseProvider('github')}
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

function GuestAvatar() {
  return (
    <span
      className="home-account-bar-avatar home-account-bar-avatar--guest"
      data-role="account-bar-avatar"
      data-placeholder="true"
      aria-hidden="true"
    >
      <svg viewBox="0 0 20 20" width="12" height="12" aria-hidden="true">
        <circle cx="10" cy="7.2" r="3.1" fill="currentColor" />
        <path
          d="M4.2 16.2c.9-3.1 2.8-4.4 5.8-4.4s4.9 1.3 5.8 4.4"
          fill="currentColor"
        />
      </svg>
    </span>
  );
}

function SignedInAvatar({ user }: { user: AccountBarUser }) {
  const url = user.avatar_url?.trim() ?? '';
  const [broken, setBroken] = useState(false);

  useEffect(() => {
    setBroken(false);
  }, [url, user.user_id]);

  if (url && !broken) {
    return (
      <img
        className="home-account-bar-avatar"
        data-role="account-bar-avatar"
        src={url}
        alt=""
        onError={() => setBroken(true)}
      />
    );
  }

  return (
    <span
      className="home-account-bar-avatar home-account-bar-avatar--placeholder"
      data-role="account-bar-avatar"
      data-placeholder="true"
      aria-hidden="true"
    >
      {avatarInitial(user)}
    </span>
  );
}

function signedInLabel(user: AccountBarUser | null): string {
  if (!user?.user_id) return 'Sign in';
  return user.display_name?.trim() || user.email?.trim() || 'Signed in';
}

function avatarInitial(user: AccountBarUser): string {
  const source = user.display_name?.trim() || user.email?.trim() || 'L';
  return Array.from(source)[0]?.toUpperCase() || 'L';
}

function callSafely(fn: () => void) {
  try {
    fn();
  } catch {
    /* keep the bar clickable */
  }
}
