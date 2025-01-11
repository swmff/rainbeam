export default function ClickOutside(element: HTMLElement, callback: any) {
    function click(event: any) {
        if (!element.contains(event.target)) {
            callback();
        }
    }

    document.body.addEventListener("click", click);

    return {
        update(callback_: any) {
            callback = callback_;
        },
        destroy() {
            document.body.removeEventListener("click", click);
        }
    };
}
