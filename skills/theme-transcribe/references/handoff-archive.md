# Write local draft

Load after `$TRANSCRIBE_CTL route`.

## When

After `route` stdout `ok` is true.

## Payload

| Field | Source |
|-------|--------|
| Primary | `route` stdout `primary` (add document header if missing) |

DONE when that file exists.

```text
> ✅ theme-transcribe complete
> 📄 local：<absolute primary path>
```
