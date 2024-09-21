#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("starting app");
    launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        head::Link {
            rel: "stylesheet",
            href: "https://cdnjs.cloudflare.com/ajax/libs/bulma/1.0.2/css/bulma.min.css"
        }
        head::Link {
            rel: "stylesheet",
            href: "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.6.0/css/all.min.css"
        }
        section { class: "section",
            div { class: "container",
                h1 { class: "title", "hololive OCG Deck Converter" }
                div { class: "block",
                    "Convert your hOCG deck between many formats like: Deck Log, holoDelta, HoloDuel, and Tabletop Simulator."
                }
                Home {}
                h1 { class: "title is-4", "Deck content" }
                div { class: "is-flex is-flex-wrap-wrap",
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                    div {
                        figure { class: "image m-1", style: "width: 10rem",
                            img { src: "https://qrimpuff.github.io/hocg-fan-sim-assets/img/hSD01/hSD01-009_R.webp" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        div { class: "columns is-desktop",
            div { class: "column is-6",
                form { class: "box",
                    div { class: "field",
                        label { class: "label", "Deck Source" }
                        div { class: "control",
                            div { class: "select",
                                select {
                                    option { "Deck Log" }
                                    option { "holoDelta" }
                                    option { "HoloDuel" }
                                    option { "Tabletop Simulator (by Noodlebrain)" }
                                    option { "I don't know..." }
                                }
                            }
                        }
                    }

                    div { class: "field",
                        div { class: "control",
                            div { class: "file",
                                label { class: "file-label",
                                    input {
                                        r#type: "file",
                                        name: "resume",
                                        class: "file-input"
                                    }
                                    span { class: "file-cta",
                                        span { class: "file-icon",
                                            i { class: "fa-solid fa-upload" }
                                        }
                                        span { class: "file-label", " Choose a file… " }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "field",
                        label { class: "label", "Url" }
                        div { class: "control",
                            input {
                                placeholder: "Text input",
                                r#type: "text",
                                class: "input"
                            }
                        }
                    }

                    div { class: "field",
                        label { class: "label", "Json" }
                        div { class: "control",
                            textarea {
                                placeholder: "e.g. Hello world",
                                class: "textarea"
                            }
                        }
                    }

                    div { class: "field",
                        label { class: "label", "Export format" }
                        div { class: "control",
                            div { class: "select",
                                select {
                                    option { "Deck Log" }
                                    option { "holoDelta" }
                                    option { "HoloDuel" }
                                    option { "Tabletop Simulator (by Noodlebrain)" }
                                }
                            }
                        }
                    }

                    div { class: "field",
                        div { class: "control",
                            button { class: "button",
                                span { class: "icon",
                                    i { class: "fa-solid fa-download" }
                                }
                                span { "Download deck file" }
                            }
                        }
                    }
                    div { class: "field",
                        div { class: "control",
                            button { class: "button",
                                span { class: "icon",
                                    i { class: "fa-solid fa-upload" }
                                }
                                span { "Upload to Deck Log" }
                            }
                        }
                    }

                    div { class: "field",
                        label { class: "label", "Json" }
                        div { class: "control",
                            textarea {
                                placeholder: "e.g. Hello world",
                                class: "textarea"
                            }
                        }
                    }
                }
            }
        }
    }
}
