function dd_tabs() {
  const tabsContainers = document.querySelectorAll('.dd-tabs');

  tabsContainers.forEach(container => {
    const links = container.querySelectorAll('.dd-tabs__menu-link');
    const items = container.querySelectorAll('.dd-tabs__item');

    links.forEach(link => {
      link.addEventListener('click', function(e) {
        e.preventDefault();
        const targetId = this.getAttribute('aria-controls');
        const targetItem = container.querySelector(`#${targetId}`);

        // Remove active from all links and items in this container
        links.forEach(l => {
          l.classList.remove('-active');
          l.setAttribute('aria-selected', 'false');
        });
        items.forEach(item => {
          item.classList.remove('-active');
          item.setAttribute('aria-hidden', 'true');
        });

        // Add active to clicked link and corresponding item
        this.classList.add('-active');
        this.setAttribute('aria-selected', 'true');
        targetItem.classList.add('-active');
        targetItem.setAttribute('aria-hidden', 'false');
      });
    });
  });
}

// Initialize on initial page load
document.addEventListener('DOMContentLoaded', () => {
  dd_tabs();
});
// Fire axe after HTMX settles
document.body.addEventListener("htmx:afterSettle", function (event) {
  dd_tabs();
});
