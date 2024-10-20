
export function track_event(event, data) {
    if (window.umami && window.umami.track) {
        window.umami.track(event, data);
    }
}