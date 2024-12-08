
export function track_event(event, data) {
    // console.log(event, data);
    if (window.umami) {
        window.umami.track(event, data);
    }
}