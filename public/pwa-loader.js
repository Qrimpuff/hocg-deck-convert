const SW_PATH = '/hocg-deck-convert/sw.js';

if ('serviceWorker' in navigator) {
    const initServiceWorker = () => {
        const sendOfflineHint = () => {
            const payload = { type: 'OFFLINE_HINT', offline: !navigator.onLine };
            if (navigator.serviceWorker.controller) {
                navigator.serviceWorker.controller.postMessage(payload);
                return;
            }

            navigator.serviceWorker.ready
                .then((registration) => {
                    if (registration.active) {
                        registration.active.postMessage(payload);
                    }
                })
                .catch(() => { });
        };

        navigator.serviceWorker.addEventListener('message', (event) => {
            if (event.data && event.data.type === 'PREFETCH_PROGRESS') {
                updateProgressUI(event.data.loaded, event.data.total, event.data.size);
            }
            if (event.data && event.data.type === 'CARD_IMAGE_UPDATED') {
                showUpdatedCardImage(event.data.url, event.data.blob);
            }
            if (event.data && event.data.type === 'CARDS_DB_UPDATED') {
                // The app loads the cards again.
                window.dispatchEvent(new Event('hocg-cards-db-updated'));
                // Downloads the images of the new cards.
                if (isStandaloneModeEnabled()) {
                    navigator.serviceWorker.ready.then(prefetchCards).catch(console.error);
                }
            }
        });

        navigator.serviceWorker.register(SW_PATH).then((registration) => {
            console.log('ServiceWorker registration successful with scope: ', registration.scope);

            sendOfflineHint();
            window.addEventListener('online', sendOfflineHint);
            window.addEventListener('offline', sendOfflineHint);

            // Defaults to the installed state until the setting is changed.
            if (isStandaloneModeEnabled()) {
                console.log('Initiating card prefetch (standalone mode on)...');
                prefetchCards(registration).catch(console.error);
            }
        }).catch((err) => {
            console.log('ServiceWorker registration failed: ', err);
        });

    };

    // Register as early as practical so the SW can pre-cache the WASM referenced by the entry JS
    // during the first visit (without relying on a reload).
    if (document.readyState === 'complete' || document.readyState === 'interactive') {
        initServiceWorker();
    } else {
        window.addEventListener('DOMContentLoaded', initServiceWorker);
    }
}

// Newer versions of card images, found by the service worker after serving the cached ones.
const updatedCardImages = new Map();
// Older object URLs of an updated image, to the image URL they replaced.
const replacedCardImages = new Map();
let updatedCardImagesObserver = null;

function swapUpdatedCardImage(img) {
    const src = img.getAttribute('src');
    const objectUrl = updatedCardImages.get(replacedCardImages.get(src) || src);
    if (objectUrl && objectUrl !== src) img.src = objectUrl;
}

function showUpdatedCardImage(url, blob) {
    if (!url || !blob) return;
    // Not revoked: a lazy image not loaded yet may still point to it, and is swapped to the
    // new one below.
    const previous = updatedCardImages.get(url);
    if (previous) replacedCardImages.set(previous, url);
    updatedCardImages.set(url, URL.createObjectURL(blob));

    document.querySelectorAll('img').forEach(swapUpdatedCardImage);

    // The page keeps the old image for this URL in memory, so images shown later need the
    // swap too.
    if (!updatedCardImagesObserver) {
        updatedCardImagesObserver = new MutationObserver((mutations) => {
            for (const mutation of mutations) {
                if (mutation.type === 'attributes') {
                    if (mutation.target.tagName === 'IMG') swapUpdatedCardImage(mutation.target);
                    continue;
                }
                for (const node of mutation.addedNodes) {
                    if (node.nodeType !== Node.ELEMENT_NODE) continue;
                    if (node.tagName === 'IMG') swapUpdatedCardImage(node);
                    node.querySelectorAll('img').forEach(swapUpdatedCardImage);
                }
            }
        });
        updatedCardImagesObserver.observe(document.body, {
            subtree: true,
            childList: true,
            attributes: true,
            attributeFilter: ['src'],
        });
    }
}

function isStandaloneApp() {
    return window.matchMedia('(display-mode: standalone)').matches || window.navigator.standalone;
}

function isStandaloneModeEnabled() {
    try {
        const settings = JSON.parse(localStorage.getItem('hocg_settings') || '{}');
        if (typeof settings.standalone_mode === 'boolean') return settings.standalone_mode;
    } catch (_) {
        // Fall through to the install state.
    }
    return isStandaloneApp();
}

window.hocgStartStandaloneDownload = () => {
    if (!('serviceWorker' in navigator)) return;
    navigator.serviceWorker.ready.then(prefetchCards).catch(console.error);
};

async function prefetchCards(registration) {
    // Update first to avoid writing into an old cache.
    try {
        await registration.update();
    } catch (e) {
        console.log('SW update check failed (offline?):', e);
    }

    const serviceWorker = registration.installing
        || registration.waiting
        || registration.active
        || (await navigator.serviceWorker.ready).active;

    if (!serviceWorker) {
        console.warn('[prefetch] No service worker available for card download');
        return;
    }

    // The service worker reads the image URLs from its copy of the card database.
    const sendPrefetch = () => {
        serviceWorker.postMessage({ type: 'PREFETCH_CARDS' });
    };

    if (serviceWorker.state === 'activated') {
        sendPrefetch();
    } else {
        serviceWorker.addEventListener('statechange', (e) => {
            if (e.target.state === 'activated') {
                sendPrefetch();
            }
        });
    }
}

function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return (bytes / Math.pow(k, i)).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + ' ' + sizes[i];
}

function updateProgressUI(loaded, total, size = 0) {
    let container = document.getElementById('pwa-prefetch-progress');
    const hasDownloadedData = size > 0;

    if (loaded >= total) {
        if (container && container.style.opacity !== '0') {
            const title = container.querySelector('#pwa-progress-title');
            if (title) title.textContent = 'Offline download complete!';
            const mainBar = container.querySelector('#pwa-progress-main-bar');
            if (mainBar) {
                mainBar.classList.replace('is-info', 'is-success');
                mainBar.value = total;
            }
            const compactBar = container.querySelector('#pwa-progress-compact-bar');
            if (compactBar) {
                compactBar.classList.replace('is-info', 'is-success');
                compactBar.value = total;
                compactBar.max = total;
            }
            const text = container.querySelector('#pwa-progress-text');
            if (text) text.textContent = `${loaded} / ${total}`;

            container.style.transition = 'opacity 0.5s ease';
            setTimeout(() => {
                // Fade out and remove; no forced reload.
                container.style.opacity = '0';
                setTimeout(() => container.remove(), 600);
            }, 1500);
        } else if (container) {
            container.remove();
        }
        return;
    }

    if (!container && !hasDownloadedData) {
        return;
    }

    if (!container) {
        createProgressContainer(loaded, total, size);
        return;
    }

    const mainBar = container.querySelector('#pwa-progress-main-bar');
    const compactBar = container.querySelector('#pwa-progress-compact-bar');
    const text = container.querySelector('#pwa-progress-text');
    const sizeText = container.querySelector('#pwa-progress-size');

    if (mainBar) {
        mainBar.value = loaded;
        mainBar.max = total;
    }
    if (compactBar) {
        compactBar.value = loaded;
        compactBar.max = total;
    }
    if (text) {
        text.textContent = `${loaded} / ${total}`;
    }
    if (sizeText) {
        sizeText.textContent = formatBytes(size);
    }
}

function applyProgressLayout(container, collapsed) {
    const expanded = container.querySelector('#pwa-progress-expanded');
    const compact = container.querySelector('#pwa-progress-compact');
    const compactBar = container.querySelector('#pwa-progress-compact-bar');
    if (expanded) expanded.style.display = collapsed ? 'none' : '';
    if (compact) compact.style.display = collapsed ? 'flex' : 'none';

    // Make collapsed toast noticeably smaller.
    container.style.width = collapsed ? '220px' : '320px';
    container.style.padding = collapsed ? '0.5rem 0.625rem' : '';
    if (compactBar) compactBar.style.height = collapsed ? '0.65rem' : '';
}

function createProgressContainer(loaded, total, size = 0) {
    let container = document.getElementById('pwa-prefetch-progress');
    if (container) return;

    container = document.createElement('div');
    container.id = 'pwa-prefetch-progress';
    container.className = 'notification';
    container.style.cssText = 'position: fixed; bottom: 20px; right: 20px; width: 320px; z-index: 9999; box-shadow: 0 0.5em 1em -0.125em rgba(10,10,10,.1), 0 0px 0 1px rgba(10,10,10,.02); opacity: 0; transform: translateY(8px); transition: opacity 0.3s ease, transform 0.3s ease;';
    container.innerHTML = `
        <div id="pwa-progress-expanded">
            <div class="is-flex is-justify-content-space-between is-align-items-center mb-2">
                <p id="pwa-progress-title" class="subtitle is-6 mb-0 has-text-weight-bold">Downloading cards for offline</p>
                <a id="pwa-progress-collapse" class="tag" href="#" aria-label="Hide details" title="Hide details" style="cursor: pointer; text-decoration: none;">
                    <span class="icon is-small"><i class="fa-solid fa-chevron-down"></i></span>
                </a>
            </div>
            <progress id="pwa-progress-main-bar" class="progress is-info is-small mb-2" value="${loaded}" max="${total}"></progress>
            <div class="is-flex is-justify-content-space-between is-size-7 has-text-grey">
                <span id="pwa-progress-text">${loaded} / ${total}</span>
                <span id="pwa-progress-size">${formatBytes(size)}</span>
            </div>
        </div>
        <div id="pwa-progress-compact" class="is-align-items-center" style="display:none; gap: 0.35rem;">
            <span class="icon is-small has-text-grey" aria-hidden="true" style="width: 1rem; height: 1rem;"><i class="fa-solid fa-download"></i></span>
            <progress id="pwa-progress-compact-bar" class="progress is-info is-small mb-0" value="${loaded}" max="${total}" style="flex: 1; margin-bottom: 0; height: 0.65rem;"></progress>
            <a id="pwa-progress-expand" class="tag" href="#" aria-label="Show details" title="Show details" style="cursor: pointer; text-decoration: none;">
                <span class="icon is-small"><i class="fa-solid fa-chevron-up"></i></span>
            </a>
        </div>
    `;
    document.body.appendChild(container);

    requestAnimationFrame(() => {
        container.style.opacity = '1';
        container.style.transform = 'translateY(0)';
    });

    const collapse = container.querySelector('#pwa-progress-collapse');
    if (collapse) {
        collapse.addEventListener('click', (event) => {
            event.preventDefault();
            applyProgressLayout(container, true);
        });
    }

    const expand = container.querySelector('#pwa-progress-expand');
    if (expand) {
        expand.addEventListener('click', (event) => {
            event.preventDefault();
            applyProgressLayout(container, false);
        });
    }
}