class PartialComponent extends HTMLElement {
    static observedAttributes = ["src", "uses"];

    constructor() {
        const self = super();
        self.style.display = "contents";
    }

    attributeChangedCallback(name, _, value) {
        switch (name) {
            case "src":
                fetch(value)
                    .then((res) => res.text())
                    .then((res) => {
                        this.innerHTML = res;
                    })
                    .catch((err) => {
                        this.innerHTML = err;
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
