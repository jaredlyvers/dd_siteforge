function dd_window_scroll() {
  /**
  * Window scroll trigger. Bind to scrollEnd to use.
  * example. jQuery(window).bind('scrollEnd', function () {});
  **/
  jQuery(window).scroll(function () {
    if (this.scrollTO) clearTimeout(this.scrollTO);
    this.scrollTO = setTimeout(function () {
      jQuery(this).trigger('scrollEnd');
    }, 0);
  });
}

// Initialize on initial page load
document.addEventListener('DOMContentLoaded', () => {
  dd_window_scroll();
});
