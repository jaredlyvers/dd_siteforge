function dd_navigation() {

    const menu_toggle = document.querySelector('.dd-menu__toggle');
    const menu_close = document.querySelector('.dd-menu__close');
    const main_menu = document.querySelector('.navigation.-main-menu');

    if (menu_toggle) {
      menu_toggle.addEventListener('click', () => {
        // Toggle the -active class on the button itself
        menu_toggle.classList.toggle('fa-bars');
        menu_toggle.classList.toggle('fa-times');
        // Toggle the -active class on the main menu div
        main_menu.classList.toggle('-active');
        // Force close search
        // Toggle the -active class on the search element
        document.querySelector('.dd-search__toggle').classList.remove('fa-times');
        document.querySelector('.dd-search__toggle').classList.add('fa-magnifying-glass');
        document.querySelector('.dd-search').classList.remove('-active');
      });
    }

    if (menu_close) {
      // Used if there is a close button as part of the menu when open on mobile
      menu_close.addEventListener('click', () => {
        // Toggle the -active class on the button itself
        menu_toggle.classList.toggle('fa-times');
        menu_toggle.classList.toggle('fa-bars');
        // Toggle the -active class on the main menu div
        main_menu.classList.toggle('-active');
      });
    }

    // Add click event listeners to menu items with children
    const menuItemsWithChildren = document.querySelectorAll('.menu-item-has-children');
    menuItemsWithChildren.forEach(item => {
      item.addEventListener('click', (event) => {
        event.preventDefault();
        if (item.classList.contains('-active')) {
          // If already active, remove it
          item.classList.remove('-active');
          item.removeAttribute('aria-expanded');
        } else {
          // Remove -active class from all other menu items with children
          menuItemsWithChildren.forEach(otherItem => {
            otherItem.classList.remove('-active');
            otherItem.removeAttribute('aria-expanded');
          });
          // Add -active class to the clicked item
          item.classList.add('-active');
          item.setAttribute('aria-expanded', 'true');
        }
        // Set focus to the clicked item for keyboard navigation
        item.focus();
      });
    });

};

// Initialize on initial page load
document.addEventListener('DOMContentLoaded', () => {
  dd_navigation();
});
// Fire axe after HTMX settles
document.body.addEventListener("htmx:afterSettle", function (event) {
  dd_navigation();
});
