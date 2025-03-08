//! # Carp
//! Sending drawn images as questions
//!
//! Rainbeam <https://github.com/swmff/rainbeam>
(() => {
    const self = reg_ns("carp", ["app"]);

    self.CARPS = {};
    self.define("new", function ({ $ }, bind_to, read_only = false) {
        const canvas = new CarpCanvas(bind_to, read_only);
        $.CARPS[bind_to.id] = canvas;
        return canvas;
    });

    class CarpCanvas {
        #element; // HTMLElement
        #ctx; // CanvasRenderingContext2D
        #pos = { x: 0, y: 0 }; // Vec2

        STROKE_SIZE = 2;
        #stroke_size_old = 2;
        COLOR = "#000000";
        #color_old = "#000000";

        LINES = [];
        HISTORY = [];
        #line_store = [];

        onedit;
        read_only;

        /// Create a new [`CarpCanvas`]
        constructor(element, read_only) {
            this.#element = element;
            this.read_only = read_only;
        }

        /// Push #line_store to LINES
        push_state(resolution = 1) {
            // clean line_store
            const seen_points = [];
            for (const line of this.LINES) {
                for (const point of line) {
                    if (seen_points.includes(point)) {
                        const index = line.indexOf(point);
                        line.splice(index, 1);
                        continue;
                    }

                    for (const i in point) {
                        // remove null values from point
                        if (point[i] === null) {
                            point.splice(i, 1);
                        }
                    }

                    seen_points.push(point);
                }
            }

            if (this.#line_store.length === 0) {
                return;
            }

            // push
            this.LINES.push(this.#line_store);
            this.HISTORY.push(this.LINES);
            this.#line_store = [];

            if (this.onedit) {
                this.onedit(this.as_string());
            }
        }

        /// Create canvas and init context
        async create_canvas() {
            const canvas = document.createElement("canvas");

            canvas.width = "300";
            canvas.height = "200";

            this.#element.appendChild(canvas);
            this.#ctx = canvas.getContext("2d");

            if (!this.read_only) {
                // desktop
                canvas.addEventListener(
                    "mousemove",
                    (e) => {
                        this.draw_event(e);
                    },
                    false,
                );

                canvas.addEventListener(
                    "mouseup",
                    (e) => {
                        this.push_state();
                    },
                    false,
                );

                canvas.addEventListener(
                    "mousedown",
                    (e) => {
                        this.move_event(e);
                    },
                    false,
                );

                canvas.addEventListener(
                    "mouseenter",
                    (e) => {
                        this.move_event(e);
                    },
                    false,
                );

                // mobile
                canvas.addEventListener(
                    "touchmove",
                    (e) => {
                        e.preventDefault();

                        e.clientX = e.changedTouches[0].clientX;
                        e.clientY = e.changedTouches[0].clientY;

                        this.draw_event(e, true);
                    },
                    false,
                );

                canvas.addEventListener(
                    "touchstart",
                    (e) => {
                        e.preventDefault();

                        e.clientX = e.changedTouches[0].clientX;
                        e.clientY = e.changedTouches[0].clientY;

                        this.move_event(e);
                    },
                    false,
                );

                canvas.addEventListener(
                    "touchend",
                    (e) => {
                        e.preventDefault();

                        e.clientX = e.changedTouches[0].clientX;
                        e.clientY = e.changedTouches[0].clientY;

                        this.push_state();
                        this.move_event(e);
                    },
                    false,
                );

                // add controls
                const container = document.createElement("div");
                container.classList.add("flex");
                container.classList.add("justify-between");
                container.classList.add("flex-wrap");
                container.classList.add("gap-4");
                container.classList.add("carp:toolbar");
                this.#element.appendChild(container);

                const control_container = document.createElement("div");
                control_container.classList.add("flex");
                control_container.classList.add("gap-4");
                container.appendChild(control_container);

                const media_container = document.createElement("div");
                media_container.classList.add("flex");
                media_container.classList.add("gap-2");
                container.appendChild(media_container);

                // color picker
                const color_picker = document.createElement("input");
                color_picker.type = "color";
                control_container.appendChild(color_picker);

                color_picker.addEventListener("change", (e) => {
                    this.set_old_color(this.COLOR);
                    this.COLOR = e.target.value;
                    color_button.style.color = this.COLOR;
                });

                control_container.appendChild(color_picker);

                // stroke size selector
                const stroke_range = document.createElement("input");
                stroke_range.type = "range";
                stroke_range.setAttribute("min", "1");
                stroke_range.setAttribute("max", "25");
                stroke_range.setAttribute("step", "1");
                stroke_range.value = "2";
                control_container.appendChild(stroke_range);

                stroke_range.addEventListener("change", (e) => {
                    this.set_old_stroke_size(this.STROKE_SIZE);
                    this.STROKE_SIZE = e.target.value;
                });

                // media buttons
                const download_dropdown = document.createElement("div");
                download_dropdown.className = "dropdown";

                const download_dropdown_button =
                    document.createElement("button");
                download_dropdown_button.title = "Download graph";
                download_dropdown_button.setAttribute("type", "button");
                download_dropdown_button.appendChild(
                    await trigger("app::icon", ["download", "icon"]),
                );
                download_dropdown_button.addEventListener("click", (event) => {
                    trigger("app::hooks::dropdown", [event]);
                });
                download_dropdown_button.setAttribute("exclude", "dropdown");

                const inner = document.createElement("div");
                inner.className = "inner";
                inner.setAttribute("exclude", "dropdown");

                const download_as_carpgraph_button =
                    document.createElement("button");
                download_as_carpgraph_button.setAttribute("type", "button");
                download_as_carpgraph_button.appendChild(
                    await trigger("app::icon", ["file-json", "icon"]),
                );
                download_as_carpgraph_button.appendChild(
                    (() => {
                        const span = document.createElement("span");
                        span.innerText = "Download .carpgraph";
                        return span;
                    })(),
                );

                download_as_carpgraph_button.addEventListener("click", () => {
                    this.download();
                });

                const download_as_svg_button = document.createElement("button");
                download_as_svg_button.setAttribute("type", "button");
                download_as_svg_button.appendChild(
                    await trigger("app::icon", ["code-xml", "icon"]),
                );
                download_as_svg_button.appendChild(
                    (() => {
                        const span = document.createElement("span");
                        span.innerText = "Download .svg";
                        return span;
                    })(),
                );

                download_as_svg_button.addEventListener("click", () => {
                    this.download_svg();
                });

                inner.appendChild(download_as_carpgraph_button);
                inner.appendChild(download_as_svg_button);
                download_dropdown.appendChild(download_dropdown_button);
                media_container.appendChild(download_dropdown);

                const upload_button = document.createElement("button");
                upload_button.title = "Upload graph";
                upload_button.appendChild(
                    await trigger("app::icon", ["upload", "icon"]),
                );

                download_dropdown.appendChild(inner);
                media_container.appendChild(upload_button);
                upload_button.setAttribute("type", "button");
                upload_button.addEventListener("click", () => {
                    const input = document.createElement("input");
                    input.type = "file";
                    input.accept = ".carpgraph";

                    input.addEventListener("change", async () => {
                        if (input.files.length === 0) {
                            return;
                        }

                        const file = input.files[0];
                        const text = await file.text();
                        this.from_string(text);
                    });

                    input.click();
                    input.remove();
                });
            }
        }

        /// Resize the canvas
        resize(size) {
            this.#ctx.canvas.width = size.x;
            this.#ctx.canvas.height = size.y;
        }

        /// Clear the canvas
        clear() {
            const canvas = this.#ctx.canvas;
            this.#ctx.clearRect(0, 0, canvas.width, canvas.height);
        }

        /// Set the old color
        set_old_color(value) {
            this.#color_old = value;
        }

        /// Set the old stroke_size
        set_old_stroke_size(value) {
            this.#stroke_size_old = value;
        }

        /// Update position (from event)
        move_event(e) {
            const rect = this.#ctx.canvas.getBoundingClientRect();

            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;

            this.move({ x, y });
        }

        /// Update position
        move(pos) {
            this.#pos.x = pos.x;
            this.#pos.y = pos.y;
        }

        /// Draw on the canvas (from event)
        draw_event(e, mobile = false) {
            if (e.buttons !== 1 && mobile === false) return;
            const rect = this.#ctx.canvas.getBoundingClientRect();

            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;

            this.draw({ x, y });
        }

        /// Draw on the canvas
        draw(pos, skip_line_store = false) {
            this.#ctx.beginPath();

            this.#ctx.lineWidth = this.STROKE_SIZE;
            this.#ctx.strokeStyle = this.COLOR;
            this.#ctx.lineCap = "round";

            this.#ctx.moveTo(this.#pos.x, this.#pos.y);
            this.move(pos);
            this.#ctx.lineTo(this.#pos.x, this.#pos.y);

            if (!skip_line_store) {
                // yes flooring the values will make the image SLIGHTLY different,
                // but it also saves THOUSANDS of characters
                const point = [
                    Math.floor(this.#pos.x),
                    Math.floor(this.#pos.y),
                    // only include these values if they changed
                    this.#color_old !== this.COLOR ? this.COLOR : null,
                    this.#stroke_size_old !== this.STROKE_SIZE
                        ? this.STROKE_SIZE
                        : null,
                ];

                for (const i in point) {
                    // remove null values from point
                    if (point[i] === null) {
                        point.splice(i, 1);
                    }
                }

                this.#line_store.push(point);

                if (this.#color_old !== this.COLOR) {
                    // we've already seen it once, time to update it
                    this.set_old_color(this.COLOR);
                }

                if (this.#stroke_size_old !== this.STROKE_SIZE) {
                    this.set_old_stroke_size(this.STROKE_SIZE);
                }
            }

            this.#ctx.stroke();
        }

        /// Create blob and get URL
        as_blob() {
            const blob = this.#ctx.canvas.toBlob();
            return URL.createObjectURL(blob);
        }

        /// Create JSON representation of the graph
        as_json() {
            this.push_state();
            return {
                // Canvas info
                i: {
                    // Canvas width
                    w: this.#ctx.canvas.width,
                    // Canvas height
                    h: this.#ctx.canvas.height,
                },
                // Canvas data
                d: this.LINES,
            };
        }

        /// Export lines as string
        as_string() {
            return JSON.stringify(this.as_json());
        }

        /// Import string as lines
        from_string(input) {
            let parsed = JSON.parse(input);

            if (Array.isArray(parsed)) {
                // legacy format
                parsed = { d: parsed };
            } else {
                // new format, includes width and height
                if (parsed.i && parsed.i.w && parsed.i.h) {
                    this.resize({ x: parsed.i.w, y: parsed.i.h });
                }
            }

            this.LINES = parsed.d;
            const rendered = [];

            // lines format:
            // [[[x, y, Option<color>, Option<stroke_size>], ...], ...]
            for (const line of this.LINES) {
                if (line[0]) {
                    // if line has at least one point, go ahead and start at that
                    // point for drawing
                    this.move({ x: line[0][0], y: line[0][1] });
                }

                for (const point of line) {
                    if (rendered.includes(point)) {
                        // we've already seen this, let's skip it
                        continue;
                    }

                    const x = point[0];
                    const y = point[1];
                    rendered.push(point);

                    if (point[2]) {
                        if (point[2].startsWith("#")) {
                            this.COLOR = point[2] || "#000000";
                        } else {
                            this.STROKE_SIZE = point[2] || 2;
                        }
                    }

                    if (point[3]) {
                        this.STROKE_SIZE = point[3] || 2;
                    }

                    this.draw({ x, y }, true);
                }
            }
        }

        /// Download image as `.carpgraph`
        download() {
            const string = this.as_string();
            const blob = new Blob([string], { type: "image/carpgraph" });
            const url = URL.createObjectURL(blob);

            const anchor = document.createElement("a");
            anchor.href = url;
            anchor.setAttribute("download", "image.carpgraph");
            document.body.appendChild(anchor);
            anchor.click();
            anchor.remove();

            URL.revokeObjectURL(url);
        }

        /// Download image as `.svg`
        download_svg() {
            fetch("/api/v0/util/carpsvg", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(this.as_json()),
            })
                .then((res) => res.text())
                .then((res) => {
                    const blob = new Blob([res], { type: "image/svg+xml" });
                    const url = URL.createObjectURL(blob);

                    const anchor = document.createElement("a");
                    anchor.href = url;
                    anchor.setAttribute("download", "image.svg");
                    document.body.appendChild(anchor);
                    anchor.click();
                    anchor.remove();

                    URL.revokeObjectURL(url);
                });
        }
    }
})();
