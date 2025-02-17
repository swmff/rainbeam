/// The location of an element as represented by array indexes.
class ElementPointer {
    position = [];

    constructor(element) {
        if (element) {
            const pos = [];

            let target = element;
            while (target.parentElement) {
                const parent = target.parentElement;

                // push index
                pos.push(Array.from(parent.children).indexOf(target) || 0);

                // update target
                if (parent.id === "editor") {
                    break;
                }

                target = parent;
            }

            this.position = pos.reverse(); // indexes are added in reverse order because of how we traverse
        } else {
            this.position = [];
        }
    }

    get() {
        return this.position;
    }

    resolve(json, minus = 0) {
        let out = json;

        if (this.position.length === 1) {
            // this is the first element (this.position === [0])
            return out;
        }

        const pos = this.position.slice(1, this.position.length); // the first one refers to the root element

        for (let i = 0; i < minus; i++) {
            pos.pop();
        }

        for (const idx of pos) {
            const child = ((out || { children: [] }).children || [])[idx];

            if (!child) {
                break;
            }

            out = child;
        }

        return out;
    }
}

const EMPTY_COMPONENT = { component: "empty", options: {}, children: [] };

function copy_fields(from, to) {
    for (const field of Object.entries(from)) {
        to[field[0]] = field[1];
    }

    return to;
}

class LayoutEditor {
    element;
    json;
    current = {};
    pointer = new ElementPointer();

    /// Create a new [`LayoutEditor`].
    constructor(element, json) {
        this.element = element;
        this.json = json;

        if (this.json.json) {
            // biome-ignore lint/performance/noDelete: literally the only way to do this
            delete this.json.json;
        }

        element.addEventListener("click", (e) => this.click(e, this));
    }

    /// Render layout.
    render() {
        fetch("/api/v0/auth/render_layout", {
            method: "POST",
            body: JSON.stringify({
                layout: this.json,
            }),
            headers: {
                "Content-Type": "application/json",
            },
        })
            .then((r) => r.text())
            .then((r) => {
                this.element.innerHTML = r;

                if (this.json.component !== "empty") {
                    // remove all "empty" components (if the root component isn't an empty)
                    for (const element of document.querySelectorAll(
                        '[data-component-name="empty"]',
                    )) {
                        element.remove();
                    }
                }
            });
    }

    /// Editor clicked.
    click(e, self) {
        e.stopImmediatePropagation();

        const ptr = new ElementPointer(e.target);
        self.current = ptr.resolve(self.json);
        self.pointer = ptr;

        self.dialog("element");
    }

    /// Render editor dialog.
    dialog(page = "element") {
        this.current.component = this.current.component.toLowerCase();
        const dialog = document.getElementById("editor_dialog");

        const inner = dialog.querySelector(".inner");
        inner.innerHTML = "";

        // render page
        if (
            page === "add" ||
            (page === "element" && this.current.component === "empty")
        ) {
            // add element
            inner.appendChild(
                (() => {
                    const heading = document.createElement("h3");
                    heading.innerText = "Add component";
                    return heading;
                })(),
            );

            const components = [
                [
                    "Markdown block",
                    {
                        component: "markdown",
                        options: {
                            text: "Hello, world!",
                        },
                    },
                ],
                [
                    "Flex container",
                    {
                        component: "flex",
                        options: {
                            direction: "row",
                            gap: "2",
                        },
                        children: [],
                    },
                ],
                [
                    "Profile tabs",
                    {
                        component: "tabs",
                    },
                ],
                [
                    "Profile feeds",
                    {
                        component: "feed",
                    },
                ],
                [
                    "Profile banner",
                    {
                        component: "banner",
                    },
                ],
                [
                    "Question box",
                    {
                        component: "ask",
                    },
                ],
                [
                    "Name & avatar",
                    {
                        component: "name",
                    },
                ],
                [
                    "About section",
                    {
                        component: "about",
                    },
                ],
                [
                    "Action buttons",
                    {
                        component: "actions",
                    },
                ],
            ];

            const container = document.createElement("div");
            container.className = "flex w-full gap-2 flex-wrap";

            for (const component of components) {
                container.appendChild(
                    (() => {
                        const button = document.createElement("button");

                        trigger("app:icon", ["shapes", "icon"]).then((icon) => {
                            button.prepend(icon);
                        });

                        button.appendChild(
                            (() => {
                                const span = document.createElement("span");
                                span.innerText = component[0];
                                return span;
                            })(),
                        );

                        button.addEventListener("click", () => {
                            if (
                                page === "element" &&
                                this.current.component === "empty"
                            ) {
                                // replace with component
                                copy_fields(component[1], this.current);
                            } else {
                                // add component to children
                                this.current.children.push(component[1]);
                            }

                            this.render();
                            dialog.close();
                        });

                        return button;
                    })(),
                );
            }

            inner.appendChild(container);
        } else if (page === "element") {
            // edit element
            inner.appendChild(
                (() => {
                    const heading = document.createElement("h3");
                    heading.innerText = `Edit ${this.current.component}`;
                    return heading;
                })(),
            );

            // options
            const add_option = (
                label_text,
                name,
                valid = [],
                input_element = "input",
            ) => {
                const card = document.createElement("div");
                card.className = "card w-full flex flex-col gap-2";

                const label = document.createElement("label");
                label.setAttribute("for", name);
                label.className = "w-full";
                label.innerText = label_text;

                const input = document.createElement(input_element);
                input.setAttribute("name", name);
                input.setAttribute("type", "text");

                if (input_element === "input") {
                    input.setAttribute(
                        "value",
                        (this.current.options || {})[name] || "",
                    );
                } else {
                    input.innerHTML = (this.current.options || {})[name] || "";
                }

                input.addEventListener("change", (e) => {
                    if (
                        valid.length > 0 &&
                        !valid.includes(e.target.value) &&
                        e.target.value.length > 0 // anything can be set to empty
                    ) {
                        alert(`Must be one of: ${JSON.stringify(valid)}`);
                        return;
                    }

                    if (!this.current.options) {
                        this.current.options = {};
                    }

                    this.current.options[name] =
                        e.target.value === "no" ? "" : e.target.value;
                });

                card.appendChild(label);
                card.appendChild(input);
                inner.appendChild(card);
            };

            if (this.current.component === "flex") {
                add_option("Gap", "gap", ["1", "2", "3", "4"]);
                add_option("Direction", "direction", ["row", "col"]);
                add_option("Do collapse", "collapse", ["yes", "no"]);
                add_option("Width", "width", ["full", "content"]);
                add_option("Class name", "class");
                add_option("Style", "style", [], "textarea");
            } else if (this.current.component === "markdown") {
                add_option("Content", "text", [], "textarea");
            } else if (this.current.component === "divider") {
                add_option("Class name", "class");
            }

            // action buttons
            const buttons = document.createElement("div");
            buttons.className = "card w-full flex flex-wrap gap-2";

            if (this.current.component === "flex") {
                buttons.appendChild(
                    (() => {
                        const button = document.createElement("button");

                        trigger("app:icon", ["plus", "icon"]).then((icon) => {
                            button.prepend(icon);
                        });

                        button.appendChild(
                            (() => {
                                const span = document.createElement("span");
                                span.innerText = "Add child";
                                return span;
                            })(),
                        );

                        button.addEventListener("click", () => {
                            dialog.close();
                            this.dialog("add");
                        });

                        return button;
                    })(),
                );
            }

            buttons.appendChild(
                (() => {
                    const button = document.createElement("button");

                    trigger("app:icon", ["move-up", "icon"]).then((icon) => {
                        button.prepend(icon);
                    });

                    button.appendChild(
                        (() => {
                            const span = document.createElement("span");
                            span.innerText = "Move up";
                            return span;
                        })(),
                    );

                    button.addEventListener("click", () => {
                        dialog.close();

                        const idx = this.pointer.get().pop();
                        const parent_ref = this.pointer.resolve(
                            this.json,
                        ).children;

                        if (parent_ref[idx - 1] === undefined) {
                            alert("No space to move element.");
                            return;
                        }

                        const clone = JSON.parse(JSON.stringify(this.current));
                        const other_clone = JSON.parse(
                            JSON.stringify(parent_ref[idx - 1]),
                        );

                        copy_fields(clone, parent_ref[idx - 1]); // move here to here
                        copy_fields(other_clone, parent_ref[idx]); // move there to here

                        this.render();
                    });

                    return button;
                })(),
            );

            buttons.appendChild(
                (() => {
                    const button = document.createElement("button");

                    trigger("app:icon", ["move-down", "icon"]).then((icon) => {
                        button.prepend(icon);
                    });

                    button.appendChild(
                        (() => {
                            const span = document.createElement("span");
                            span.innerText = "Move down";
                            return span;
                        })(),
                    );

                    button.addEventListener("click", () => {
                        dialog.close();

                        const idx = this.pointer.get().pop();
                        const parent_ref = this.pointer.resolve(
                            this.json,
                        ).children;

                        if (parent_ref[idx + 1] === undefined) {
                            alert("No space to move element.");
                            return;
                        }

                        const clone = JSON.parse(JSON.stringify(this.current));
                        const other_clone = JSON.parse(
                            JSON.stringify(parent_ref[idx + 1]),
                        );

                        copy_fields(clone, parent_ref[idx + 1]); // move here to here
                        copy_fields(other_clone, parent_ref[idx]); // move there to here

                        this.render();
                    });

                    return button;
                })(),
            );

            buttons.appendChild(
                (() => {
                    const button = document.createElement("button");

                    button.classList.add("red");

                    trigger("app:icon", ["trash", "icon"]).then((icon) => {
                        button.prepend(icon);
                    });

                    button.appendChild(
                        (() => {
                            const span = document.createElement("span");
                            span.innerText = "Delete";
                            return span;
                        })(),
                    );

                    button.addEventListener("click", async () => {
                        if (
                            !(await trigger("app:confirm", [
                                "Are you sure you would like to do this?",
                            ]))
                        ) {
                            return;
                        }

                        if (this.json === this.current) {
                            // this is the root element; replace with empty
                            copy_fields(EMPTY_COMPONENT, this.current);
                        } else {
                            // get parent
                            const idx = this.pointer.get().pop();
                            const ref = this.pointer.resolve(this.json);
                            // remove element
                            ref.children.splice(idx, 1);
                        }

                        this.render();
                        dialog.close();
                    });

                    return button;
                })(),
            );

            inner.appendChild(buttons);
        }

        inner.appendChild(document.createElement("hr"));
        inner.appendChild(
            (() => {
                const button = document.createElement("button");

                button.classList.add("button");
                button.classList.add("green");

                trigger("app:icon", ["check", "icon"]).then((icon) => {
                    button.prepend(icon);
                });

                button.appendChild(
                    (() => {
                        const span = document.createElement("span");
                        span.innerText = "Save";
                        return span;
                    })(),
                );

                button.addEventListener("click", () => {
                    this.render();
                    dialog.close();
                });

                return button;
            })(),
        );

        // ...
        dialog.showModal();
    }
}

define("ElementPointer", ElementPointer);
define("LayoutEditor", LayoutEditor);
