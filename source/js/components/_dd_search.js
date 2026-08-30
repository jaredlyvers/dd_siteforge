function dd_search() {

    const search_toggle = document.querySelector('.dd-search__toggle');
    const search_close = document.querySelector('.dd-search__close');
    const dd_search = document.querySelector('.dd-search');

    if (search_toggle) {
      search_toggle.addEventListener('click', () => {
        // Toggle the -active class on the button itself
        search_toggle.classList.toggle('fa-magnifying-glass');
        search_toggle.classList.toggle('fa-times');
        // Toggle the -active class on the search div
        dd_search.classList.toggle('-active');
        // Force close menu
        // Toggle the -active class on the menu element
        document.querySelector('.dd-menu__toggle').classList.remove('fa-times');
        document.querySelector('.dd-menu__toggle').classList.add('fa-bars');
        document.querySelector('.navigation.-main-menu').classList.remove('-active');
      });
    }

    if (search_close) {
      // Used if there is a close button as part of the search when open
      search_close.addEventListener('click', () => {
        // Toggle the -active class on the button itself
        search_toggle.classList.toggle('fa-times');
        search_toggle.classList.toggle('fa-magnifying-glass');
        // Toggle the -active class on the search div
        dd_search.classList.toggle('-active');
      });
    }

};

// Initialize on initial page load
document.addEventListener('DOMContentLoaded', () => {
  dd_search();
});
