/**
 * Image Zoom and Pan UI
 *
 * Handles zoom controls, pan/drag functionality, and mouse wheel zooming
 * for the lightbox image viewer. Maintains per-image zoom state persistence.
 */

// State management for zoom levels per image
const imageZoomState = new Map();

// DOM elements
const lightbox = document.getElementById('lightbox');
const lightboxContainer = document.getElementById('lightbox-container');
const lightboxToolbar = document.getElementById('lightbox-toolbar');
const lightboxImg = document.getElementById('lightbox-img');
const lightboxWrapper = document.getElementById('lightbox-image-wrapper');
const zoomOutBtn = document.getElementById('lightbox-zoom-out');
const zoomInBtn = document.getElementById('lightbox-zoom-in');
const resetZoomBtn = document.getElementById('lightbox-reset-zoom');
const fitWindowBtn = document.getElementById('lightbox-fit-window');
const zoomDisplay = document.getElementById('lightbox-zoom-display');
const zoomSlider = document.getElementById('lightbox-zoom-slider');

let currentFileId = null;
let isDragging = false;
let dragStartX = 0;
let dragStartY = 0;
let lastPanX = 0;
let lastPanY = 0;

/**
 * Get or create zoom state for a file
 */
function getZoomState(fileId) {
  if (!imageZoomState.has(fileId)) {
    imageZoomState.set(fileId, {
      zoom: 1.0,
      panX: 0,
      panY: 0,
    });
  }
  return imageZoomState.get(fileId);
}

/**
 * Update the zoom display and apply transformations
 */
async function updateZoomDisplay() {
  if (!currentFileId) return;

  const state = getZoomState(currentFileId);
  const zoomLevel = Math.round(state.zoom * 10) / 10;
  zoomDisplay.textContent = zoomLevel.toFixed(1) + 'x';
  zoomSlider.value = state.zoom;
  applyImageTransform();
}

/**
 * Apply CSS transform to image based on zoom and pan
 */
function applyImageTransform() {
  if (!currentFileId) return;

  const state = getZoomState(currentFileId);
  // Use CSS transform for smooth hardware-accelerated zoom and pan
  // scale(Z) translate(X) means: scale first, then translate in scaled space
  lightboxImg.style.transform = `scale(${state.zoom}) translate(${state.panX}px, ${state.panY}px)`;
}

/**
 * Handle zoom in command via Tauri
 */
async function handleZoomIn() {
  if (!currentFileId) return;

  try {
    const newZoom = await invoke('zoom_in', { file_id: currentFileId });
    const state = getZoomState(currentFileId);
    state.zoom = newZoom;
    state.panX = 0;
    state.panY = 0;
    await updateZoomDisplay();
  } catch (err) {
    console.error('Zoom in failed:', err);
  }
}

/**
 * Handle zoom out command via Tauri
 */
async function handleZoomOut() {
  if (!currentFileId) return;

  try {
    const newZoom = await invoke('zoom_out', { file_id: currentFileId });
    const state = getZoomState(currentFileId);
    state.zoom = newZoom;
    state.panX = 0;
    state.panY = 0;
    await updateZoomDisplay();
  } catch (err) {
    console.error('Zoom out failed:', err);
  }
}

/**
 * Handle reset zoom command via Tauri
 */
async function handleResetZoom() {
  if (!currentFileId) return;

  try {
    await invoke('reset_zoom', { file_id: currentFileId });
    const state = getZoomState(currentFileId);
    state.zoom = 1.0;
    state.panX = 0;
    state.panY = 0;
    await updateZoomDisplay();
  } catch (err) {
    console.error('Reset zoom failed:', err);
  }
}

/**
 * Handle fit to window command via Tauri
 */
async function handleFitToWindow() {
  if (!currentFileId || !lightboxImg.src) return;

  try {
    // Get image dimensions when it's loaded
    const img = new Image();
    img.onload = async () => {
      const wrapperWidth = lightboxWrapper.clientWidth;
      const wrapperHeight = lightboxWrapper.clientHeight;

      const imageWidth = img.naturalWidth;
      const imageHeight = img.naturalHeight;

      const zoomX = wrapperWidth / imageWidth;
      const zoomY = wrapperHeight / imageHeight;
      const fitZoom = Math.min(zoomX, zoomY, 15.0);

      const state = getZoomState(currentFileId);
      state.zoom = Math.max(0.1, fitZoom);
      state.panX = 0;
      state.panY = 0;

      await updateZoomDisplay();
    };
    img.src = lightboxImg.src;
  } catch (err) {
    console.error('Fit to window failed:', err);
  }
}

/**
 * Handle zoom slider input
 */
async function handleZoomSlider(e) {
  if (!currentFileId) return;

  const newZoom = parseFloat(e.target.value);
  const state = getZoomState(currentFileId);
  state.zoom = newZoom;
  state.panX = 0;
  state.panY = 0;

  await updateZoomDisplay();
}

/**
 * Handle mouse wheel zoom
 */
function handleMouseWheel(e) {
  if (!currentFileId || !lightbox.classList.contains('show')) return;

  e.preventDefault();

  const state = getZoomState(currentFileId);
  const delta = e.deltaY > 0 ? -1 : 1;

  // Zoom increment per wheel tick
  const increment = 0.1;
  const newZoom = Math.max(1.0, Math.min(15.0, state.zoom + (delta * increment)));

  state.zoom = newZoom;
  state.panX = 0;
  state.panY = 0;

  updateZoomDisplay();
}

/**
 * Handle pan start (mouse down on image)
 */
function handlePanStart(e) {
  if (!currentFileId || e.button !== 0) return; // Left mouse button only

  const state = getZoomState(currentFileId);
  if (state.zoom <= 1.0) return; // Can't pan at normal zoom

  isDragging = true;
  dragStartX = e.clientX;
  dragStartY = e.clientY;
  lastPanX = state.panX;
  lastPanY = state.panY;

  lightboxWrapper.classList.add('panning');
  e.preventDefault();
}

/**
 * Handle pan move (mouse move while dragging)
 */
function handlePanMove(e) {
  if (!isDragging || !currentFileId) return;

  const state = getZoomState(currentFileId);

  // Calculate movement in screen coordinates, convert to scaled space
  const screenDeltaX = e.clientX - dragStartX;
  const screenDeltaY = e.clientY - dragStartY;
  const scaledDeltaX = screenDeltaX / state.zoom;
  const scaledDeltaY = screenDeltaY / state.zoom;

  // Constrain panning within image bounds
  const displayWidth = lightboxImg.naturalWidth * state.zoom;
  const displayHeight = lightboxImg.naturalHeight * state.zoom;
  const wrapperWidth = lightboxWrapper.clientWidth;
  const wrapperHeight = lightboxWrapper.clientHeight;

  // Maximum pan in scaled space: how far can we translate before hitting wrapper edge
  const maxPanX = Math.max(0, (displayWidth - wrapperWidth) / 2 / state.zoom);
  const maxPanY = Math.max(0, (displayHeight - wrapperHeight) / 2 / state.zoom);

  // Apply pan constraints: pan can be positive or negative, up to maxPan
  const newPanX = lastPanX + scaledDeltaX;
  const newPanY = lastPanY + scaledDeltaY;

  state.panX = Math.max(-maxPanX, Math.min(maxPanX, newPanX));
  state.panY = Math.max(-maxPanY, Math.min(maxPanY, newPanY));

  applyImageTransform();
}

/**
 * Handle pan end (mouse up)
 */
function handlePanEnd() {
  isDragging = false;
  lightboxWrapper.classList.remove('panning');
}

/**
 * Initialize lightbox with a new image
 */
function initLightboxImage(src) {
  // Generate a file ID based on the image source
  currentFileId = 'img_' + src.replace(/[^a-zA-Z0-9]/g, '_').slice(0, 50);

  // Get or initialize zoom state for this image
  const state = getZoomState(currentFileId);

  // Load the image and apply stored zoom state
  lightboxImg.onload = () => {
    updateZoomDisplay();
  };

  lightboxImg.src = src;
  // Apply the stored zoom/pan state from initialization
  applyImageTransform();
}

/**
 * Close the lightbox
 */
function closeLightboxZoom() {
  lightbox.classList.remove('show');
  currentFileId = null;
  isDragging = false;
  lightboxWrapper.classList.remove('panning');
  lightboxImg.src = '';
}

// Event listeners for zoom controls
zoomOutBtn.addEventListener('click', handleZoomOut);
zoomInBtn.addEventListener('click', handleZoomIn);
resetZoomBtn.addEventListener('click', handleResetZoom);
fitWindowBtn.addEventListener('click', handleFitToWindow);
zoomSlider.addEventListener('input', handleZoomSlider);

// Mouse wheel zoom
lightboxWrapper.addEventListener('wheel', handleMouseWheel, { passive: false });

// Pan/drag support
lightboxImg.addEventListener('mousedown', handlePanStart);
document.addEventListener('mousemove', handlePanMove);
document.addEventListener('mouseup', handlePanEnd);

// Lightbox close handling
lightbox.addEventListener('click', (e) => {
  // Only close if clicking on the background, not the toolbar or image wrapper
  if (e.target === lightbox) {
    closeLightboxZoom();
  }
});

// Keyboard support
document.addEventListener('keydown', (e) => {
  if (!lightbox.classList.contains('show')) return;

  switch (e.key) {
    case 'Escape':
      e.stopPropagation();
      closeLightboxZoom();
      break;
    case '+':
    case '=':
      e.preventDefault();
      handleZoomIn();
      break;
    case '-':
    case '_':
      e.preventDefault();
      handleZoomOut();
      break;
    case '0':
      e.preventDefault();
      handleResetZoom();
      break;
    case 'f':
    case 'F':
      if (!e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        handleFitToWindow();
      }
      break;
  }
}, true);

// Export for use by main script
window.initLightboxImage = initLightboxImage;
window.closeLightboxZoom = closeLightboxZoom;
