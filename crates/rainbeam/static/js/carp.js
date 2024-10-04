//! # Carp
//! Sending drawn images as questions
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
        COLOR = "#000000";

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
            if (this.#line_store.length === 0) {
                return;
            }

            // clean line_store
            const seen_points = [];
            for (const line of this.LINES) {
                for (const point of line) {
                    if (seen_points.includes(point)) {
                        const index = line.indexOf(point);
                        line.splice(index, 1);
                        continue;
                    }

                    seen_points.push(point);
                }
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
                if (!("ontouchstart" in document.documentElement)) {
                    // desktop
                    canvas.addEventListener("mousemove", (e) => {
                        this.draw_event(e);
                    });

                    canvas.addEventListener("mouseup", (e) => {
                        this.push_state();
                    });

                    canvas.addEventListener("mousedown", (e) => {
                        this.move_event(e);
                    });

                    canvas.addEventListener("mouseenter", (e) => {
                        this.move_event(e);
                    });
                } else {
                    // mobile
                    canvas.addEventListener("touchmove", (e) => {
                        e.preventDefault();

                        e.clientX = e.changedTouches[0].clientX;
                        e.clientY = e.changedTouches[0].clientY;

                        this.draw_event(e);
                    });

                    canvas.addEventListener("touchstart", (e) => {
                        e.preventDefault();

                        e.clientX = e.changedTouches[0].clientX;
                        e.clientY = e.changedTouches[0].clientY;

                        this.move_event(e);
                    });

                    canvas.addEventListener("touchleave", (e) => {
                        e.preventDefault();

                        e.clientX = e.changedTouches[0].clientX;
                        e.clientY = e.changedTouches[0].clientY;

                        this.push_state();
                        this.move_event(e);
                    });
                }
            }
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
        draw_event(e) {
            if (e.buttons !== 1) return;
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
                this.#line_store.push([
                    Math.floor(this.#pos.x),
                    Math.floor(this.#pos.y),
                ]);
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
            return JSON.stringify(this.LINES);
        }

        /// Import string as lines
        from_string(input) {
            this.LINES = JSON.parse(input);
            let rendered = [];

            // lines format:
            // [[[x, y], ...], ...]
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

                    this.draw({ x, y }, true);
                }
            }
        }
    }
})();
