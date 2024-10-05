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
                container.classList.add("gap-4");
                this.#element.appendChild(container);

                const color_picker = document.createElement("input");
                color_picker.type = "color";
                container.appendChild(color_picker);

                color_picker.addEventListener("change", (e) => {
                    this.set_old_color(this.COLOR);
                    this.COLOR = e.target.value;
                });

                const stroke_range = document.createElement("input");
                stroke_range.type = "range";
                stroke_range.setAttribute("min", "2");
                stroke_range.setAttribute("max", "10");
                stroke_range.setAttribute("step", "1");
                stroke_range.value = "2";
                container.appendChild(stroke_range);

                stroke_range.addEventListener("change", (e) => {
                    this.set_old_stroke_size(this.STROKE_SIZE);
                    this.STROKE_SIZE = e.target.value;
                });
            }
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
            return JSON.stringify(this.LINES);
        }

        /// Import string as lines
        from_string(input) {
            this.LINES = JSON.parse(input);
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
    }
})();
