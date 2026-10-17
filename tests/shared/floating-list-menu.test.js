// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { positionFloatingListMenu } from '../../frontend/src/shared/floating-list-menu.ts';

function rect({ left, top, width, height }) {
  return {
    left,
    top,
    width,
    height,
    right: left + width,
    bottom: top + height,
    x: left,
    y: top,
    toJSON() {},
  };
}

describe('positionFloatingListMenu', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('sizes to content instead of locking to the trigger width', () => {
    const menu = document.createElement('div');
    menu.style.position = 'fixed';
    const option = document.createElement('button');
    option.textContent = 'Dialogue';
    menu.appendChild(option);
    document.body.appendChild(menu);

    positionFloatingListMenu(menu, rect({ left: 20, top: 40, width: 48, height: 22 }));

    expect(menu.style.minWidth).toBe('48px');
    expect(menu.style.width).toBe('max-content');
    expect(menu.style.width).not.toBe('48px');
  });

  it('shifts left when content would overflow the viewport', () => {
    const menu = document.createElement('div');
    menu.style.position = 'fixed';
    document.body.appendChild(menu);
    menu.getBoundingClientRect = () => rect({ left: 120, top: 0, width: 200, height: 80 });
    const innerWidth = window.innerWidth;
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 180 });

    positionFloatingListMenu(menu, rect({ left: 120, top: 40, width: 40, height: 22 }), 400);

    expect(menu.style.left).toBe('8px');
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: innerWidth });
  });
});
