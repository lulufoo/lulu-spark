import { Fragment, type KeyboardEvent } from 'react';
import {
  CHIP_LABEL_CLASS,
  chipLabelText,
  parseReferences,
  referenceKindByName,
} from '../references/index.ts';

/** A user message: plain text stays text; registered references show as clickable chips. */
export function UserMessageText({ text }: { text: string }) {
  return (
    <>
      {parseReferences(text).map((segment, index) => {
        if (segment.type === 'text') return <Fragment key={index}>{segment.text}</Fragment>;
        const kind = referenceKindByName(segment.kind);
        const open = () => {
          void kind?.open(segment.id);
        };
        return (
          <span
            key={index}
            className="home-ref-chip"
            role="link"
            tabIndex={0}
            title={segment.title}
            data-ref-kind={segment.kind}
            data-ref-id={segment.id}
            onClick={open}
            onKeyDown={(event: KeyboardEvent<HTMLSpanElement>) => {
              if (event.key === 'Enter') open();
            }}
          >
            <span className={CHIP_LABEL_CLASS}>{chipLabelText(segment.kind, segment.title)}</span>
          </span>
        );
      })}
    </>
  );
}
