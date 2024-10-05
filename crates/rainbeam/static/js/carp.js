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

                    for (let i in point) {
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
        create_canvas() {
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
                color_picker.style.display = "none";
                control_container.appendChild(color_picker);

                color_picker.addEventListener("change", (e) => {
                    this.set_old_color(this.COLOR);
                    this.COLOR = e.target.value;
                    color_button.style.color = this.COLOR;
                });

                const color_button = document.createElement("button");
                color_button.classList.add("primary");
                color_button.title = "Select color";
                color_button.innerHTML = `<svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 16 16"
    width="16"
    height="16"
    class="icon"
>
    <path
        d="M11.134 1.535c.7-.509 1.416-.942 2.076-1.155.649-.21 1.463-.267 2.069.34.603.601.568 1.411.368 2.07-.202.668-.624 1.39-1.125 2.096-1.011 1.424-2.496 2.987-3.775 4.249-1.098 1.084-2.132 1.839-3.04 2.3a3.744 3.744 0 0 1-1.055 3.217c-.431.431-1.065.691-1.657.861-.614.177-1.294.287-1.914.357A21.151 21.151 0 0 1 .797 16H.743l.007-.75H.749L.742 16a.75.75 0 0 1-.743-.742l.743-.008-.742.007v-.054a21.25 21.25 0 0 1 .13-2.284c.067-.647.187-1.287.358-1.914.17-.591.43-1.226.86-1.657a3.746 3.746 0 0 1 3.227-1.054c.466-.893 1.225-1.907 2.314-2.982 1.271-1.255 2.833-2.75 4.245-3.777ZM1.62 13.089c-.051.464-.086.929-.104 1.395.466-.018.932-.053 1.396-.104a10.511 10.511 0 0 0 1.668-.309c.526-.151.856-.325 1.011-.48a2.25 2.25 0 1 0-3.182-3.182c-.155.155-.329.485-.48 1.01a10.515 10.515 0 0 0-.309 1.67Zm10.396-10.34c-1.224.89-2.605 2.189-3.822 3.384l1.718 1.718c1.21-1.205 2.51-2.597 3.387-3.833.47-.662.78-1.227.912-1.662.134-.444.032-.551.009-.575h-.001V1.78c-.014-.014-.113-.113-.548.027-.432.14-.995.462-1.655.942Zm-4.832 7.266-.001.001a9.859 9.859 0 0 0 1.63-1.142L7.155 7.216a9.7 9.7 0 0 0-1.161 1.607c.482.302.889.71 1.19 1.192Z"
    ></path>
</svg>`;

                color_button.addEventListener("click", () => {
                    color_picker.click();
                });

                control_container.appendChild(color_button);

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
                const download_button = document.createElement("button");
                download_button.title = "Download graph";
                download_button.innerHTML = `<svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 16 16"
    width="16"
    height="16"
    class="icon"
>
    <path
        d="M2.75 14A1.75 1.75 0 0 1 1 12.25v-2.5a.75.75 0 0 1 1.5 0v2.5c0 .138.112.25.25.25h10.5a.25.25 0 0 0 .25-.25v-2.5a.75.75 0 0 1 1.5 0v2.5A1.75 1.75 0 0 1 13.25 14Z"
    ></path>
    <path
        d="M7.25 7.689V2a.75.75 0 0 1 1.5 0v5.689l1.97-1.969a.749.749 0 1 1 1.06 1.06l-3.25 3.25a.749.749 0 0 1-1.06 0L4.22 6.78a.749.749 0 1 1 1.06-1.06l1.97 1.969Z"
    ></path>
</svg>`;

                media_container.appendChild(download_button);
                download_button.setAttribute("type", "button");
                download_button.addEventListener("click", () => {
                    this.download();
                });

                const upload_button = document.createElement("button");
                upload_button.title = "Upload graph";
                upload_button.innerHTML = `<svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 16 16"
    width="16"
    height="16"
    class="icon"
>
    <path
        d="M2.75 14A1.75 1.75 0 0 1 1 12.25v-2.5a.75.75 0 0 1 1.5 0v2.5c0 .138.112.25.25.25h10.5a.25.25 0 0 0 .25-.25v-2.5a.75.75 0 0 1 1.5 0v2.5A1.75 1.75 0 0 1 13.25 14Z"
    ></path>
    <path
        d="M11.78 4.72a.749.749 0 1 1-1.06 1.06L8.75 3.811V9.5a.75.75 0 0 1-1.5 0V3.811L5.28 5.78a.749.749 0 1 1-1.06-1.06l3.25-3.25a.749.749 0 0 1 1.06 0l3.25 3.25Z"
    ></path>
</svg>`;

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
                let point = [
                    Math.floor(this.#pos.x),
                    Math.floor(this.#pos.y),
                    // only include these values if they changed
                    this.#color_old !== this.COLOR ? this.COLOR : null,
                    this.#stroke_size_old !== this.STROKE_SIZE
                        ? this.STROKE_SIZE
                        : null,
                ];

                for (let i in point) {
                    // remove null values from point
                    if (point[i] === null) {
                        point.splice(i, 1);
                    }
                }

                this.#line_store.push(point);

                if (this.#color_old != this.COLOR) {
                    // we've already seen it once, time to update it
                    this.set_old_color(this.COLOR);
                }

                if (this.#stroke_size_old != this.STROKE_SIZE) {
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

        /// Export lines as string
        as_string() {
            this.push_state();
            return JSON.stringify({
                // Canvas info
                i: {
                    // Canvas width
                    w: this.#ctx.canvas.width,
                    // Canvas height
                    h: this.#ctx.canvas.height,
                },
                // Canvas data
                d: this.LINES,
            });
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
            let rendered = [];

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
                        this.COLOR = point[2] || "#000000";
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
    }
})();
