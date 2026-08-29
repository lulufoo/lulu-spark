package com.lulu.workbench.android.chat.ui

/**
 * Chat IME has one owner. Window resize and Compose IME padding must not
 * both consume the keyboard height — that is the send-time jump.
 *
 * API 30+: Compose owns IME. The window stays still ([ChatImeOwner.Compose]).
 * Older APIs: the window resizes ([ChatImeOwner.Window]); Compose pads
 * navigation bars only.
 *
 * Send never changes IME visibility or field enabled. IME hides only when
 * the user taps the transcript or toolbar.
 */
internal const val ComposeImeOwnerSdk = 30

internal enum class ChatImeOwner {
    Window,
    Compose,
}

internal fun chatImeOwner(sdk: Int): ChatImeOwner =
    if (sdk >= ComposeImeOwnerSdk) ChatImeOwner.Compose else ChatImeOwner.Window
