// @ts-nocheck
(() => {
    const self = reg_ns("codemirror");

    // create_editor
    self.define(
        "create_editor",
        function (
            _,
            bind_to,
            value,
            placeholder,
            global = "editor",
            mode = "markdown",
            inputStyle = "contenteditable"
        ) {
            const editor = CodeMirror(bind_to, {
                value: value || "",
                mode,
                lineWrapping: true,
                autoCloseBrackets: true,
                autofocus: true,
                viewportMargin: Number.POSITIVE_INFINITY,
                inputStyle,
                highlightFormatting: false,
                fencedCodeBlockHighlighting: false,
                xml: false,
                smartIndent: false,
                placeholder,
                extraKeys: {
                    Home: "goLineLeft",
                    End: "goLineRight",
                    Enter: (cm) => {
                        cm.replaceSelection("\n");
                    }
                }
            });

            globalThis[global] = editor;

            // ...
            for (const element of Array.from(
                document.querySelectorAll(".CodeMirror-code")
            )) {
                element.setAttribute("spellcheck", "true");
            }

            // return
            return editor;
        },
        ["object", "string", "string"]
    );

    // tabs
    self.define("init_tabs", ({ markdown }) => {
        const text_button = document.getElementById("text_button");
        const text_tab = document.getElementById("text_tab");

        const preview_button = document.getElementById("preview_button");
        const preview_tab = document.getElementById("preview_tab");

        if (text_button && preview_button) {
            text_button.addEventListener("click", () => {
                preview_button.classList.remove("primary");
                text_button.classList.add("primary");

                preview_tab.style.display = "none";
                text_tab.style.display = "block";
            });

            preview_button.addEventListener("click", async () => {
                text_button.classList.remove("primary");
                preview_button.classList.add("primary");

                text_tab.style.display = "none";
                preview_tab.style.display = "block";

                // render
                preview_tab.innerHTML = "";
                preview_tab.innerHTML = await (
                    await fetch("/api/v1/pages/_app/render", {
                        method: "POST",
                        headers: {
                            "Content-Type": "application/json"
                        },
                        body: JSON.stringify({
                            content: globalThis.editor.getValue()
                        })
                    })
                ).text();
            });
        }
    });
})();
