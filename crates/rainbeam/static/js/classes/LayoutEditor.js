/// Copy all the fields from one object to another.
function copy_fields(from, to) {
    for (const field of Object.entries(from)) {
        to[field[0]] = field[1];
    }

    return to;
}

/// Simple template components.
const COMPONENT_TEMPLATES = {
    EMPTY_COMPONENT: { component: "empty", options: {}, children: [] },
    FLEX_DEFAULT: {
        component: "flex",
        options: {
            direction: "row",
            gap: "2",
        },
        children: [],
    },
    FLEX_SIMPLE_ROW: {
        component: "flex",
        options: {
            direction: "row",
            gap: "2",
            width: "full",
        },
        children: [],
    },
    FLEX_SIMPLE_COL: {
        component: "flex",
        options: {
            direction: "col",
            gap: "2",
            width: "full",
        },
        children: [],
    },
    FLEX_MOBILE_COL: {
        component: "flex",
        options: {
            collapse: "yes",
            gap: "2",
            width: "full",
        },
        children: [],
    },
    MARKDOWN_DEFAULT: {
        component: "markdown",
        options: {
            text: "Hello, world!",
        },
    },
    MARKDOWN_CARD: {
        component: "markdown",
        options: {
            class: "card w-full",
            text: "Hello, world!",
        },
    },
};

/// All available components with their label and JSON representation.
const COMPONENTS = [
    [
        "Markdown block",
        COMPONENT_TEMPLATES.MARKDOWN_DEFAULT,
        [["Card", COMPONENT_TEMPLATES.MARKDOWN_CARD]],
    ],
    [
        "Flex container",
        COMPONENT_TEMPLATES.FLEX_DEFAULT,
        [
            ["Simple rows", COMPONENT_TEMPLATES.FLEX_SIMPLE_ROW],
            ["Simple columns", COMPONENT_TEMPLATES.FLEX_SIMPLE_COL],
            ["Mobile columns", COMPONENT_TEMPLATES.FLEX_MOBILE_COL],
        ],
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
    [
        "CSS stylesheet",
        {
            component: "style",
            options: {
                data: "",
            },
        },
    ],
];

// preload icons
trigger("app::icon", ["shapes"]);
trigger("app::icon", ["type"]);
trigger("app::icon", ["plus"]);
trigger("app::icon", ["move-up"]);
trigger("app::icon", ["move-down"]);
trigger("app::icon", ["trash"]);
trigger("app::icon", ["arrow-left"]);
trigger("app::icon", ["x"]);

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

/// The layout editor controller.
class LayoutEditor {
    element;
    json;
    tree = "";
    current = { component: "empty" };
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
        element.addEventListener("mouseover", (e) => {
            e.stopImmediatePropagation();
            const ptr = new ElementPointer(e.target);

            if (document.getElementById("position")) {
                document.getElementById(
                    "position",
                ).parentElement.style.display = "flex";

                document.getElementById("position").innerText = ptr
                    .get()
                    .join(".");
            }
        });

        this.render();
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
            .then((r) => r.json())
            .then((r) => {
                this.element.innerHTML = r.block;
                this.tree = r.tree;

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
        trigger("app::hooks::dropdown.close");

        const ptr = new ElementPointer(e.target);
        self.current = ptr.resolve(self.json);
        self.pointer = ptr;

        if (document.getElementById("current_position")) {
            document.getElementById(
                "current_position",
            ).parentElement.style.display = "flex";

            document.getElementById("current_position").innerText = ptr
                .get()
                .join(".");
        }

        for (const element of document.querySelectorAll(
            ".layout_editor_block.active",
        )) {
            element.classList.remove("active");
        }

        e.target.classList.add("active");
        self.screen("element");
    }

    /// Open sidebar.
    open() {
        document.getElementById("editor_sidebar").classList.add("open");
        document.getElementById("editor").style.transform = "scale(0.8)";
    }

    /// Close sidebar.
    close() {
        document.getElementById("editor_sidebar").style.animation =
            "0.2s ease-in-out forwards to_left";

        setTimeout(() => {
            document.getElementById("editor_sidebar").classList.remove("open");
            document.getElementById("editor_sidebar").style.animation =
                "0.2s ease-in-out forwards from_right";
        }, 250);

        document.getElementById("editor").style.transform = "scale(1)";
    }

    /// Render editor dialog.
    screen(page = "element", data = {}) {
        this.current.component = this.current.component.toLowerCase();

        const sidebar = document.getElementById("editor_sidebar");
        sidebar.innerHTML = "";

        // render page
        if (
            page === "add" ||
            (page === "element" && this.current.component === "empty")
        ) {
            // add element
            sidebar.appendChild(
                (() => {
                    const heading = document.createElement("h3");
                    heading.innerText = data.add_title || "Add component";
                    return heading;
                })(),
            );

            sidebar.appendChild(document.createElement("hr"));

            const container = document.createElement("div");
            container.className = "flex w-full gap-2 flex-wrap";

            for (const component of data.components || COMPONENTS) {
                container.appendChild(
                    (() => {
                        const button = document.createElement("button");
                        button.classList.add("secondary");

                        trigger("app::icon", [
                            data.icon || "shapes",
                            "icon",
                        ]).then((icon) => {
                            button.prepend(icon);
                        });

                        button.appendChild(
                            (() => {
                                const span = document.createElement("span");
                                span.innerText = `${component[0]}${component[2] ? ` (${component[2].length + 1})` : ""}`;
                                return span;
                            })(),
                        );

                        button.addEventListener("click", () => {
                            if (component[2]) {
                                // render presets
                                return this.screen(page, {
                                    back: ["add", {}],
                                    add_title: "Select preset",
                                    components: [
                                        ["Default", component[1]],
                                        ...component[2],
                                    ],
                                    icon: "type",
                                });
                            }

                            // no presets
                            if (
                                page === "element" &&
                                this.current.component === "empty"
                            ) {
                                // replace with component
                                copy_fields(component[1], this.current);
                            } else {
                                // add component to children
                                this.current.children.push(
                                    structuredClone(component[1]),
                                );
                            }

                            this.render();
                            this.close();
                        });

                        return button;
                    })(),
                );
            }

            sidebar.appendChild(container);
        } else if (page === "element") {
            // edit element
            const name = document.createElement("div");
            name.className = "flex flex-col gap-2";

            name.appendChild(
                (() => {
                    const heading = document.createElement("h3");
                    heading.innerText = `Edit ${this.current.component}`;
                    return heading;
                })(),
            );

            name.appendChild(
                (() => {
                    const pos = document.createElement("div");
                    pos.className = "notification w-content";
                    pos.innerText = this.pointer.get().join(".");
                    return pos;
                })(),
            );

            sidebar.appendChild(name);
            sidebar.appendChild(document.createElement("hr"));

            // options
            const options = document.createElement("div");
            options.className = "card flex flex-col gap-2 w-full";

            const add_option = (
                label_text,
                name,
                valid = [],
                input_element = "input",
            ) => {
                const card = document.createElement("details");
                card.className = "w-full";

                const summary = document.createElement("summary");
                summary.className = "w-full";

                const label = document.createElement("label");
                label.setAttribute("for", name);
                label.className = "w-full";
                label.innerText = label_text;
                label.style.cursor = "pointer";

                label.addEventListener("click", () => {
                    // bubble to summary click
                    summary.click();
                });

                const input_box = document.createElement("div");
                input_box.style.paddingLeft = "1rem";
                input_box.style.borderLeft =
                    "solid 2px var(--color-super-lowered)";

                const input = document.createElement(input_element);
                input.id = name;
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

                if ((this.current.options || {})[name]) {
                    // open details if a value is set
                    card.setAttribute("open", "");
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

                summary.appendChild(label);
                card.appendChild(summary);
                input_box.appendChild(input);
                card.appendChild(input_box);
                options.appendChild(card);
            };

            sidebar.appendChild(options);

            if (this.current.component === "flex") {
                add_option("Gap", "gap", ["1", "2", "3", "4"]);
                add_option("Direction", "direction", ["row", "col"]);
                add_option("Do collapse", "collapse", ["yes", "no"]);
                add_option("Width", "width", ["full", "content"]);
                add_option("Class name", "class");
                add_option("Unique ID", "id");
                add_option("Style", "style", [], "textarea");
            } else if (this.current.component === "markdown") {
                add_option("Content", "text", [], "textarea");
                add_option("Class name", "class");
            } else if (this.current.component === "divider") {
                add_option("Class name", "class");
            } else if (this.current.component === "style") {
                add_option("Style data", "data", [], "textarea");
            } else {
                options.remove();
            }

            // action buttons
            const buttons = document.createElement("div");
            buttons.className = "card w-full flex flex-wrap gap-2";

            if (this.current.component === "flex") {
                buttons.appendChild(
                    (() => {
                        const button = document.createElement("button");

                        trigger("app::icon", ["plus", "icon"]).then((icon) => {
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
                            this.screen("add");
                        });

                        return button;
                    })(),
                );
            }

            buttons.appendChild(
                (() => {
                    const button = document.createElement("button");

                    trigger("app::icon", ["move-up", "icon"]).then((icon) => {
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

                        this.close();
                        this.render();
                    });

                    return button;
                })(),
            );

            buttons.appendChild(
                (() => {
                    const button = document.createElement("button");

                    trigger("app::icon", ["move-down", "icon"]).then((icon) => {
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

                        this.close();
                        this.render();
                    });

                    return button;
                })(),
            );

            buttons.appendChild(
                (() => {
                    const button = document.createElement("button");
                    button.classList.add("red");

                    trigger("app::icon", ["trash", "icon"]).then((icon) => {
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
                            !(await trigger("app::confirm", [
                                "Are you sure you would like to do this?",
                            ]))
                        ) {
                            return;
                        }

                        if (this.json === this.current) {
                            // this is the root element; replace with empty
                            copy_fields(
                                COMPONENT_TEMPLATES.EMPTY_COMPONENT,
                                this.current,
                            );
                        } else {
                            // get parent
                            const idx = this.pointer.get().pop();
                            const ref = this.pointer.resolve(this.json);
                            // remove element
                            ref.children.splice(idx, 1);
                        }

                        this.render();
                        this.close();
                    });

                    return button;
                })(),
            );

            sidebar.appendChild(buttons);
        } else if (page === "tree") {
            sidebar.innerHTML = this.tree;
        }

        sidebar.appendChild(document.createElement("hr"));

        const buttons = document.createElement("div");
        buttons.className = "flex gap-2 flex-wrap";

        if (data.back) {
            buttons.appendChild(
                (() => {
                    const button = document.createElement("button");
                    button.className = "secondary";

                    trigger("app::icon", ["arrow-left", "icon"]).then(
                        (icon) => {
                            button.prepend(icon);
                        },
                    );

                    button.appendChild(
                        (() => {
                            const span = document.createElement("span");
                            span.innerText = "Back";
                            return span;
                        })(),
                    );

                    button.addEventListener("click", () => {
                        this.screen(...data.back);
                    });

                    return button;
                })(),
            );
        }

        buttons.appendChild(
            (() => {
                const button = document.createElement("button");
                button.className = "red secondary";

                trigger("app::icon", ["x", "icon"]).then((icon) => {
                    button.prepend(icon);
                });

                button.appendChild(
                    (() => {
                        const span = document.createElement("span");
                        span.innerText = "Close";
                        return span;
                    })(),
                );

                button.addEventListener("click", () => {
                    this.render();
                    this.close();
                });

                return button;
            })(),
        );

        sidebar.appendChild(buttons);

        // ...
        this.open();
    }
}

define("ElementPointer", ElementPointer);
define("LayoutEditor", LayoutEditor);
