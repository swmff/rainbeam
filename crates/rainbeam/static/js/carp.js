//! # Carp
//! Sending drawn images as questions
//!
//! Rainbeam <https://github.com/swmff/rainbeam>
(() => {
    const self = reg_ns("carp", ["app"]);

    const END_OF_HEADER = 0x1a;
    const COLOR = 0x1b;
    const SIZE = 0x2b;
    const LINE = 0x3b;
    const POINT = 0x4b;
    const EOF = 0x1f;

    function enc(s, as = "guess") {
        if ((as === "guess" && typeof s === "number") || as === "u32") {
            // encode u32
            const view = new DataView(new ArrayBuffer(16));
            view.setUint32(0, s);
            return new Uint8Array(view.buffer).slice(0, 4);
        }

        if (as === "u16") {
            // encode u16
            const view = new DataView(new ArrayBuffer(16));
            view.setUint16(0, s);
            return new Uint8Array(view.buffer).slice(0, 2);
        }

        // encode string
        const encoder = new TextEncoder();
        return encoder.encode(s);
    }

    function dec(as, from) {
        if (as === "u32") {
            // decode u32
            const view = new DataView(new Uint8Array(from).buffer);
            return view.getUint32(0);
        }

        if (as === "u16") {
            // decode u16
            const view = new DataView(new Uint8Array(from).buffer);
            return view.getUint16(0);
        }

        // decode string
        const decoder = new TextDecoder();
        return decoder.decode(from);
    }

    function lpad(size, input) {
        if (input.length === size) {
            return input;
        }

        for (let i = 0; i < size - (input.length - 1); i++) {
            input = [0, ...input];
        }

        return input;
    }

    self.enc = enc;
    self.dec = dec;
    self.lpad = lpad;

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

        COMMANDS = [];
        HISTORY = [];
        #cmd_store = [];

        onedit;
        read_only;

        /// Create a new [`CarpCanvas`]
        constructor(element, read_only) {
            this.#element = element;
            this.read_only = read_only;
        }

        /// Push #line_store to LINES
        push_state() {
            this.COMMANDS = [...this.COMMANDS, ...this.#cmd_store];
            this.#cmd_store = [];

            this.HISTORY.push(this.COMMANDS);

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
                        this.#cmd_store.push({
                            type: "Line",
                            data: [],
                        });

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

                        this.#cmd_store.push({
                            type: "Line",
                            data: [],
                        });

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
                    await trigger("app::icon", ["binary", "icon"]),
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
                        const bytes = await file.bytes();
                        this.from_bytes(Array.from(bytes));
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
                ];

                if (this.#color_old !== this.COLOR) {
                    this.#cmd_store.push({
                        type: "Color",
                        data: enc(this.COLOR.replace("#", "")),
                    });
                }

                if (this.#stroke_size_old !== this.STROKE_SIZE) {
                    this.#cmd_store.push({
                        type: "Size",
                        data: lpad(2, enc(this.STROKE_SIZE, "u16")), // u16
                    });
                }

                this.#cmd_store.push({
                    type: "Point",
                    data: [
                        // u32
                        ...lpad(4, enc(point[0])),
                        ...lpad(4, enc(point[1])),
                    ],
                });

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

        /// Create Carp2 representation of the graph
        as_carp2() {
            // most stuff should have an lpad of 4 to make sure it's a u32 (4 bytes)
            const header = [
                ...enc("CG"),
                ...enc("02"),
                ...lpad(4, enc(this.#ctx.canvas.width)),
                ...lpad(4, enc(this.#ctx.canvas.height)),
                END_OF_HEADER,
            ];

            // build commands
            const commands = [];
            commands.push(COLOR);
            commands.push(...enc("000000"));
            commands.push(SIZE);
            commands.push(...lpad(4, enc(2)).slice(2));

            for (const command of this.COMMANDS) {
                // this is `impl Into<Vec<u8>> for Command`
                switch (command.type) {
                    case "Point":
                        commands.push(POINT);
                        break;

                    case "Line":
                        commands.push(LINE);
                        break;

                    case "Color":
                        commands.push(COLOR);
                        break;

                    case "Size":
                        commands.push(SIZE);
                        break;
                }

                commands.push(...command.data);
            }

            // return
            return [...header, ...commands, EOF];
        }

        /// Export as SVG
        async as_svg() {
            return await (
                await fetch("/api/v0/util/carpsvg", {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/octet-stream",
                    },
                    body: new Uint8Array(this.as_carp2()),
                })
            ).text();
        }

        /// Export lines as string
        as_string() {
            return JSON.stringify(this.COMMANDS);
        }

        /// From an array of bytes
        from_bytes(input) {
            this.clear();

            let idx = -1;
            function next() {
                idx += 1;
                return [idx, input[idx]];
            }

            function select_bytes(count) {
                // select_bytes! macro
                const data = [];
                let seen_bytes = 0;

                let [_, byte] = next();
                while (byte !== undefined) {
                    seen_bytes += 1;
                    data.push(byte);

                    if (seen_bytes === count) {
                        break;
                    }

                    [_, byte] = next();
                }

                return data;
            }

            // everything past this is just a reverse implementation of carp2.rs in js
            const commands = [];
            const dimensions = { x: 0, y: 0 };
            let in_header = true;
            let seen_point = false;
            let byte_buffer = [];

            let [i, byte] = next();
            while (byte !== undefined) {
                switch (byte) {
                    case END_OF_HEADER:
                        in_header = false;
                        break;

                    case COLOR:
                        {
                            const data = select_bytes(6);
                            commands.push({
                                type: "Color",
                                data,
                            });
                            this.COLOR = `#${dec("string", new Uint8Array(data))}`;
                        }
                        break;

                    case SIZE:
                        {
                            const data = select_bytes(2);
                            commands.push({
                                type: "Size",
                                data,
                            });
                            this.STROKE_SIZE = dec("u16", data);
                        }
                        break;

                    case POINT:
                        {
                            const data = select_bytes(8);
                            commands.push({
                                type: "Point",
                                data,
                            });

                            const point = {
                                x: dec("u32", data.slice(0, 4)),
                                y: dec("u32", data.slice(4, 8)),
                            };

                            if (!seen_point) {
                                // this is the FIRST POINT that has been seen...
                                // we need to start drawing from here to avoid a line
                                // from 0,0 to the point
                                this.move(point);
                                seen_point = true;
                            }

                            this.draw(point, true);
                        }
                        break;

                    case LINE:
                        // each line starts at a new place (probably)
                        seen_point = false;
                        break;

                    case EOF:
                        break;

                    default:
                        if (in_header) {
                            if (0 <= i < 2) {
                                // tag
                            } else if (2 <= i < 4) {
                                //version
                            } else if (4 <= i < 8) {
                                // width
                                byte_buffer.push(byte);

                                if (i === 7) {
                                    dimensions.x = dec("u32", byte_buffer);
                                    byte_buffer = [];
                                }
                            } else if (8 <= i < 12) {
                                // height
                                byte_buffer.push(byte);

                                if (i === 7) {
                                    dimensions.y = dec("u32", byte_buffer);
                                    byte_buffer = [];
                                    this.resize(dimensions); // update canvas
                                }
                            }
                        } else {
                            // misc byte
                            console.log(`extraneous byte at ${i}`);
                        }

                        break;
                }

                // ...
                [i, byte] = next();
            }

            return commands;
        }

        /// Download image as `.carpgraph`
        download() {
            const blob = new Blob([new Uint8Array(this.as_carp2())], {
                type: "image/carpgraph",
            });

            const url = URL.createObjectURL(blob);

            const anchor = document.createElement("a");
            anchor.href = url;
            anchor.setAttribute("download", "image.carpgraph");
            document.body.appendChild(anchor);
            anchor.click();
            anchor.remove();

            URL.revokeObjectURL(url);
        }

        /// Download image as `.carpgraph1`
        download_json() {
            const string = this.as_string();
            const blob = new Blob([string], { type: "application/json" });
            const url = URL.createObjectURL(blob);

            const anchor = document.createElement("a");
            anchor.href = url;
            anchor.setAttribute("download", "image.carpgraph_json");
            document.body.appendChild(anchor);
            anchor.click();
            anchor.remove();

            URL.revokeObjectURL(url);
        }

        /// Download image as `.svg`
        download_svg() {
            this.as_svg().then((res) => {
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
