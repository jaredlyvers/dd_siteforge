function dd_modal() {
  // Check if any element with data-modal exists
  if (document.querySelector("[data-modal]")) {
    // Handle open / close buttons
    const modal_open_buttons = document.querySelectorAll("[data-modal-open]");
    modal_open_buttons.forEach((modal_open_button) => {
      const modalId = modal_open_button.getAttribute("data-id");
      const modal = document.getElementById(modalId);
      const modal_close_button = modal.querySelector("[data-modal-close]");
      if (modal) {
        // Handle open button
        modal_open_button.addEventListener("click", () => {
          modal.showModal();
        });
        // Handle close button
        modal_close_button.addEventListener("click", () => {
          modal.close();
        });
      }
    });

    // Close on outside click for all modals
    const modals = document.querySelectorAll("[data-modal]");
    modals.forEach((modal) => {
      modal.addEventListener("click", (e) => {
        if (e.target === modal) {
          modal.close();
        }
      });
    });
  }
}

// Initialize on initial page load
document.addEventListener("DOMContentLoaded", () => {
  dd_modal();
});
// Fire after HTMX settles
document.body.addEventListener("htmx:afterSettle", function (event) {
  dd_modal();
});
