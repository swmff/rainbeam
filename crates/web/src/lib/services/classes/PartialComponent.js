let observer;

function create_observer() {
    return new IntersectionObserver(
        (entries) => {
            for (const entry of entries) {
                if (!entry.isIntersecting) {
                    continue;
                }

                if (!entry.target.loaded) {
                    entry.target.fetch_src(entry.target.getAttribute("src"));
                }
            }
        },
        {
            root: document.body,
            rootMargin: "0px",
            threshold: 1.0,
        },
    );
}

observer = undefined;
document.documentElement.addEventListener("turbo:load", () => {
    if (observer) {
        observer.disconnect();
        observer = undefined;
    }

    observer = create_observer();
});

class PartialComponent extends HTMLElement {
    static observedAttributes = ["src", "uses"];
    loaded;

    constructor() {
        const self = super();

        (async () => {
            const svg = await trigger("app:icon", ["loader-circle", "icon"]);
            self.innerHTML = `<div class="spinner constant flex">${svg.outerHTML}</div>`;
        })();
    }

    error() {
        this.innerHTML =
            '<div class="markdown-alert-warning">Could not display component.</div>';
    }

    fetch_src(value) {
        fetch(value)
            .then((res) => res.text())
            .then((res) => {
                if (res.includes("<title>Uh oh!")) {
                    // bad request
                    this.error();
                    return;
                }

                if (!this.getAttribute("outerhtml")) {
                    this.innerHTML = `<div style="animation: grow 1 0.25s forwards running">${res}</div>`;
                } else {
                    // "complete" replace
                    const dom = new DOMParser().parseFromString(
                        res,
                        "text/html",
                    );

                    this.replaceWith(...dom.body.children);
                }

                if (globalThis[`lib:${value}`]) {
                    // load finished
                    globalThis[`lib:${value}`]();
                }

                this.loaded = true;
                this.setAttribute("loaded", this.loaded);

                setTimeout(() => {
                    if (!this.getAttribute("uses")) {
                        return;
                    }

                    for (const hook of this.getAttribute("uses").split(",")) {
                        trigger(hook);
                    }
                }, 15);
            })
            .catch((err) => {
                this.error();
                console.error(err);
            });
    }

    attributeChangedCallback(name, old, value) {
        switch (name) {
            case "src":
                if (
                    old === value &&
                    this.getAttribute("loaded", loaded) !== true
                ) {
                    console.log("partial already loaded with unchanged src");
                    return;
                }

                this.loaded = false;
                this.setAttribute("loaded", this.loaded);

                if (!this.getAttribute("instant")) {
                    // load when in view
                    if (observer === undefined) {
                        observer = create_observer();
                    }

                    setTimeout(() => {
                        if (!observer) {
                            // how??
                            console.log("???");
                            window.location.reload();
                            return;
                        }

                        observer.observe(this);
                    }, 500);
                } else {
                    // load after a second
                    setTimeout(() => {
                        this.fetch_src(value);
                    }, 250);
                }

                break;

            case "uses":
                break;

            default:
                return;
        }
    }
}

customElements.define("include-partial", PartialComponent);
define("PartialComponent", PartialComponent);
