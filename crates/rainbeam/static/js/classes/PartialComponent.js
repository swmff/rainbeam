class PartialComponent extends HTMLElement {
    static observedAttributes = ["src", "uses"];
    loaded;

    constructor() {
        const self = super();
        self.innerHTML = '<div class="spinner">🎣</div>';
    }

    error() {
        this.innerHTML =
            '<div class="markdown-alert-warning">Could not display component.</div>';
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
                fetch(value)
                    .then((res) => res.text())
                    .then((res) => {
                        if (res.includes("<title>Uh oh!")) {
                            // neospring error
                            this.error();
                            return;
                        }

                        this.innerHTML = `<div style="animation: grow 1 0.25s forwards running">${res}</div>`;

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

                            for (const hook of this.getAttribute("uses").split(
                                ",",
                            )) {
                                trigger(hook);
                            }
                        }, 15);
                    })
                    .catch((err) => {
                        this.error();
                        console.error(err);
                    });

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
