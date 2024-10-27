class PartialComponent extends HTMLElement {
    static observedAttributes = ["src", "uses"];
    loaded;

    attributeChangedCallback(name, _, value) {
        switch (name) {
            case "src":
                this.loaded = false;
                fetch(value)
                    .then((res) => res.text())
                    .then((res) => {
                        this.innerHTML = res;

                        if (globalThis[`lib:${value}`]) {
                            // load finished
                            globalThis[`lib:${value}`]();
                        }

                        this.loaded = true;
                    })
                    .catch((err) => {
                        this.innerHTML =
                            "<span>Could not display component.</span>";
                        console.error(err);
                    });

                break;

            case "uses":
                setTimeout(() => {
                    for (const hook of value.split(",")) {
                        trigger(hook);
                    }
                }, 500);

                break;

            default:
                return;
        }
    }
}

customElements.define("include-partial", PartialComponent);
define("PartialComponent", PartialComponent);
