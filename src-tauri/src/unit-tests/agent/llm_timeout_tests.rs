use std::time::Duration;

use super::{connect_timeout_for, DEFAULT_TIMEOUT, LLM_CONNECT_TIMEOUT};

#[test]
fn connect_and_idle_timeouts_are_different_values() {
    assert_eq!(LLM_CONNECT_TIMEOUT, Duration::from_secs(30));
    assert_eq!(DEFAULT_TIMEOUT, Duration::from_secs(120));
    assert_eq!(connect_timeout_for(DEFAULT_TIMEOUT), LLM_CONNECT_TIMEOUT);
    assert_ne!(connect_timeout_for(DEFAULT_TIMEOUT), DEFAULT_TIMEOUT);
}

#[test]
fn connect_timeout_never_exceeds_a_shorter_caller_timeout() {
    let short = Duration::from_millis(200);
    assert_eq!(connect_timeout_for(short), short);
}

mod total_timeout {
    use std::io::{self, Read};
    use std::thread;
    use std::time::Duration;

    use crate::agent::llm::record::CallTrace;
    use crate::agent::llm::stream::assemble_sse;
    use crate::agent::llm::{LlmCallOutcome, LlmError};

    /// Hands out one SSE chunk per read, waiting `pause` first; ends with `end`.
    struct SlowStream {
        chunks: usize,
        pause: Duration,
        end: Option<io::ErrorKind>,
    }

    impl Read for SlowStream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            thread::sleep(self.pause);
            if self.chunks == 0 {
                return match self.end {
                    Some(kind) => Err(io::Error::from(kind)),
                    None => {
                        let done = b"data: [DONE]\n\n";
                        buf[..done.len()].copy_from_slice(done);
                        self.end = Some(io::ErrorKind::UnexpectedEof);
                        Ok(done.len())
                    }
                };
            }
            self.chunks -= 1;
            let chunk = b"data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n";
            buf[..chunk.len()].copy_from_slice(chunk);
            Ok(chunk.len())
        }
    }

    fn run(stream: SlowStream, total_limit: Duration) -> LlmCallOutcome {
        let mut trace = CallTrace::start();
        assemble_sse(stream, &mut None, &mut trace, total_limit)
    }

    fn content_of(outcome: &LlmCallOutcome) -> String {
        outcome
            .snapshot
            .as_ref()
            .and_then(|message| message.content.clone())
            .unwrap_or_default()
    }

    #[test]
    fn a_stream_inside_both_limits_completes() {
        let stream = SlowStream { chunks: 3, pause: Duration::from_millis(5), end: None };
        let outcome = run(stream, Duration::from_secs(5));
        assert_eq!(outcome.error, None);
        assert_eq!(content_of(&outcome), "aaa");
    }

    #[test]
    fn an_idle_stream_is_a_timeout_and_keeps_what_arrived() {
        let stream = SlowStream {
            chunks: 2,
            pause: Duration::from_millis(5),
            end: Some(io::ErrorKind::TimedOut),
        };
        let outcome = run(stream, Duration::from_secs(5));
        assert_eq!(outcome.error, Some(LlmError::Timeout));
        assert_eq!(content_of(&outcome), "aa");
    }

    #[test]
    fn a_stream_that_keeps_sending_past_the_total_limit_is_a_total_timeout() {
        let stream = SlowStream { chunks: 1000, pause: Duration::from_millis(20), end: None };
        let outcome = run(stream, Duration::from_millis(150));
        assert_eq!(outcome.error, Some(LlmError::TotalTimeout));
        let received = content_of(&outcome);
        assert!(!received.is_empty() && received.len() < 1000, "kept {} chunks", received.len());
    }
}
