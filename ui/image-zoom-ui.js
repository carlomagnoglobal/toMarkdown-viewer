/**
 * Image Zoom and Pan
 *
 * Provides a reusable zoom/pan controller used by two surfaces:
 *   1. the fullscreen lightbox, and
 *   2. the in-pane image viewer (.image-viewer in the content pane).
 *
 * Both surfaces size their <img> with CSS (max-width/max-height: 100%), so the
 * image is already fitted to its container at transform scale 1.0. That makes
 * 1.0 the "fit" level: no fit-scale math is needed, and panning is meaningful
 * exactly when scale > 1.0.
 */

// Persisted zoom/pan per image source, shared across both surfaces.
const imageZoomState = new Map();

const DEFAULT_MIN_ZOOM = 0.1;
const DEFAULT_MAX_ZOOM = 15.0;
const ZOOM_STEP = 0.1;
// Pointer travel (px) past which a mouseup counts as a drag, not a click.
const DRAG_THRESHOLD = 4;

function stateKeyFor(src) {
  return 'img_' + String(src || '').replace(/[^a-zA-Z0-9]/g, '_').slice(0, 50);
}

function getZoomState(key) {
  if (!imageZoomState.has(key)) {
    imageZoomState.set(key, { zoom: 1.0, panX: 0, panY: 0 });
  }
  return imageZoomState.get(key);
}

/**
 * Size the image is actually painted at, at transform scale 1.0.
 *
 * Both surfaces size their <img> box to the container and let object-fit:contain
 * letterbox the image inside it, so clientWidth/Height describe the *box*, not
 * the image. fitScale is how far the natural pixels were scaled to fit, which is
 * what turns a transform scale into a true magnification percentage.
 *
 * naturalWidth comes from the browser, so it is populated even for files whose
 * dimensions the Rust side failed to probe (SVGs report 0x0 there).
 */
function getPaintedSize(img) {
  const nw = img.naturalWidth;
  const nh = img.naturalHeight;
  if (!nw || !nh) {
    // Broken or not-yet-decoded image: degrade to treating the box as the image.
    return { width: img.clientWidth, height: img.clientHeight, fitScale: 1 };
  }
  const fitScale = Math.min(img.clientWidth / nw, img.clientHeight / nh);
  return { width: nw * fitScale, height: nh * fitScale, fitScale };
}

/**
 * Wire zoom + pan onto an image inside a wrapper.
 *
 * @param {HTMLImageElement} img     the image to transform
 * @param {HTMLElement}      wrapper the element that receives wheel events
 * @param {number}  [minZoom]
 * @param {number}  [maxZoom]
 * @param {Function} [onClick]  called with the image src on a real click (not a drag)
 * @param {Function} [onChange] called with the zoom level whenever it changes
 * @returns {{destroy:Function, setSource:Function, zoomIn:Function, zoomOut:Function,
 *            reset:Function, fit:Function, setZoom:Function, getZoom:Function}}
 */
function createZoomController({
  img,
  wrapper,
  minZoom = DEFAULT_MIN_ZOOM,
  maxZoom = DEFAULT_MAX_ZOOM,
  onClick = null,
  onChange = null,
}) {
  if (!img || !wrapper) return null;

  let key = stateKeyFor(img.getAttribute('src'));
  let isDragging = false;
  let movedDistance = 0;
  let dragStartX = 0;
  let dragStartY = 0;
  let lastPanX = 0;
  let lastPanY = 0;

  function clamp(z) {
    return Math.max(minZoom, Math.min(maxZoom, z));
  }

  function applyTransform() {
    const s = getZoomState(key);
    // scale() first, then translate() in the scaled coordinate space.
    img.style.transform = `scale(${s.zoom}) translate(${s.panX}px, ${s.panY}px)`;
  }

  function displayPercent() {
    // True magnification against natural pixels, not the transform scale: a 200px
    // image filled into a 1000px pane reads 500%, which is the informative number.
    return Math.round(getPaintedSize(img).fitScale * getZoomState(key).zoom * 100);
  }

  function commit() {
    applyTransform();
    if (onChange) onChange(getZoomState(key).zoom, displayPercent());
  }

  function setZoom(z, { resetPan = true } = {}) {
    const s = getZoomState(key);
    s.zoom = clamp(z);
    if (resetPan || s.zoom <= 1.0) { s.panX = 0; s.panY = 0; }
    commit();
  }

  function zoomIn()  { setZoom(getZoomState(key).zoom + ZOOM_STEP); }
  function zoomOut() { setZoom(getZoomState(key).zoom - ZOOM_STEP); }
  // CSS already fits the image at 1.0, so "fit" and "reset" coincide.
  function reset()   { setZoom(1.0); }

  function onWheel(e) {
    e.preventDefault();
    const dir = e.deltaY > 0 ? -1 : 1;
    setZoom(getZoomState(key).zoom + dir * ZOOM_STEP);
  }

  function onMouseDown(e) {
    if (e.button !== 0) return;
    const s = getZoomState(key);
    movedDistance = 0;
    if (s.zoom <= 1.0) return; // nothing to pan at or below fit
    isDragging = true;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    lastPanX = s.panX;
    lastPanY = s.panY;
    img.classList.add('panning');
    wrapper.classList.add('panning');
    e.preventDefault();
  }

  function onMouseMove(e) {
    if (!isDragging) return;
    const s = getZoomState(key);

    const screenDeltaX = e.clientX - dragStartX;
    const screenDeltaY = e.clientY - dragStartY;
    movedDistance = Math.max(movedDistance, Math.abs(screenDeltaX) + Math.abs(screenDeltaY));

    // translate() runs inside the scaled space, so convert screen px to that space.
    const scaledDeltaX = screenDeltaX / s.zoom;
    const scaledDeltaY = screenDeltaY / s.zoom;

    // Scale from the painted size, not clientWidth: object-fit:contain letterboxes
    // the image inside a full-size box, so clientWidth is the container.
    const painted = getPaintedSize(img);
    const displayWidth = painted.width * s.zoom;
    const displayHeight = painted.height * s.zoom;
    const maxPanX = Math.max(0, (displayWidth - wrapper.clientWidth) / 2 / s.zoom);
    const maxPanY = Math.max(0, (displayHeight - wrapper.clientHeight) / 2 / s.zoom);

    s.panX = Math.max(-maxPanX, Math.min(maxPanX, lastPanX + scaledDeltaX));
    s.panY = Math.max(-maxPanY, Math.min(maxPanY, lastPanY + scaledDeltaY));
    applyTransform();
  }

  function onMouseUp() {
    if (!isDragging) return;
    isDragging = false;
    img.classList.remove('panning');
    wrapper.classList.remove('panning');
  }

  function onClickImg(e) {
    // Suppress the click that terminates a pan gesture.
    if (movedDistance > DRAG_THRESHOLD) { movedDistance = 0; return; }
    if (onClick) { e.preventDefault(); onClick(img.src); }
  }

  function onDblClick(e) {
    e.preventDefault();
    reset();
  }

  // fitScale depends on both the decoded size and the container size, so the
  // percentage is wrong until the image decodes and goes stale on every resize.
  function onLoad() { commit(); }

  wrapper.addEventListener('wheel', onWheel, { passive: false });
  img.addEventListener('mousedown', onMouseDown);
  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
  img.addEventListener('dblclick', onDblClick);
  img.addEventListener('load', onLoad);
  if (onClick) img.addEventListener('click', onClickImg);

  let resizeObserver = null;
  if (typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(() => commit());
    resizeObserver.observe(wrapper);
  }

  return {
    destroy() {
      wrapper.removeEventListener('wheel', onWheel);
      img.removeEventListener('mousedown', onMouseDown);
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      img.removeEventListener('dblclick', onDblClick);
      img.removeEventListener('load', onLoad);
      if (onClick) img.removeEventListener('click', onClickImg);
      if (resizeObserver) { resizeObserver.disconnect(); resizeObserver = null; }
    },
    setSource(src) {
      key = stateKeyFor(src);
      img.src = src;
      commit();
    },
    zoomIn,
    zoomOut,
    reset,
    fit: reset,
    setZoom,
    getZoom() { return getZoomState(key).zoom; },
  };
}

// ---- Lightbox wiring ----

const lightbox = document.getElementById('lightbox');
const lightboxImg = document.getElementById('lightbox-img');
const lightboxWrapper = document.getElementById('lightbox-image-wrapper');
const zoomOutBtn = document.getElementById('lightbox-zoom-out');
const zoomInBtn = document.getElementById('lightbox-zoom-in');
const resetZoomBtn = document.getElementById('lightbox-reset-zoom');
const fitWindowBtn = document.getElementById('lightbox-fit-window');
const zoomDisplay = document.getElementById('lightbox-zoom-display');
const zoomSlider = document.getElementById('lightbox-zoom-slider');

const lightboxZoom = createZoomController({
  img: lightboxImg,
  wrapper: lightboxWrapper,
  onChange: (zoom) => {
    const rounded = Math.round(zoom * 10) / 10;
    if (zoomDisplay) zoomDisplay.textContent = rounded.toFixed(1) + 'x';
    if (zoomSlider) zoomSlider.value = zoom;
  },
});

if (lightboxZoom) {
  // These run the same local zoom math as the wheel and slider. They previously
  // went through invoke('zoom_in', { file_id }) — Tauri exposes a Rust `file_id`
  // parameter to JS as `fileId`, so those calls always rejected and the buttons
  // silently did nothing.
  if (zoomOutBtn) zoomOutBtn.addEventListener('click', () => lightboxZoom.zoomOut());
  if (zoomInBtn) zoomInBtn.addEventListener('click', () => lightboxZoom.zoomIn());
  if (resetZoomBtn) resetZoomBtn.addEventListener('click', () => lightboxZoom.reset());
  if (fitWindowBtn) fitWindowBtn.addEventListener('click', () => lightboxZoom.fit());
  if (zoomSlider) {
    zoomSlider.addEventListener('input', (e) => lightboxZoom.setZoom(parseFloat(e.target.value)));
  }
}

function initLightboxImage(src) {
  if (lightboxZoom) lightboxZoom.setSource(src);
}

function closeLightboxZoom() {
  lightbox.classList.remove('show');
  lightboxImg.src = '';
}

lightbox.addEventListener('click', (e) => {
  // Only the backdrop closes; the toolbar and image wrapper do not.
  if (e.target === lightbox) closeLightboxZoom();
});

document.addEventListener('keydown', (e) => {
  if (!lightbox.classList.contains('show') || !lightboxZoom) return;

  switch (e.key) {
    case 'Escape':
      e.stopPropagation();
      closeLightboxZoom();
      break;
    case '+':
    case '=':
      e.preventDefault();
      lightboxZoom.zoomIn();
      break;
    case '-':
    case '_':
      e.preventDefault();
      lightboxZoom.zoomOut();
      break;
    case '0':
      e.preventDefault();
      lightboxZoom.reset();
      break;
    case 'f':
    case 'F':
      if (!e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        lightboxZoom.fit();
      }
      break;
  }
}, true);

// Exports for the main script
window.createZoomController = createZoomController;
window.initLightboxImage = initLightboxImage;
window.closeLightboxZoom = closeLightboxZoom;
