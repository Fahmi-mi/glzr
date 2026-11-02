export interface MenuPosition {
  x: number;
  y: number;
}

export function clampMenuCoordinates(
  clientX: number,
  clientY: number,
  menuWidth: number,
  menuHeight: number,
  windowWidth: number,
  windowHeight: number,
  padding = 8
): MenuPosition {
  let x = clientX;
  let y = clientY;

  if (x + menuWidth > windowWidth - padding) {
    x = Math.max(padding, windowWidth - menuWidth - padding);
  } else {
    x = Math.max(padding, x);
  }

  if (y + menuHeight > windowHeight - padding) {
    y = Math.max(padding, windowHeight - menuHeight - padding);
  } else {
    y = Math.max(padding, y);
  }

  return { x, y };
}
