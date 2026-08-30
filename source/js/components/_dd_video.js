
function dd_video() {
  // Work grid video hover setting
  document.querySelectorAll('.dd-work_grid__item').forEach(container => {
    const video = container.querySelector('video');
    if (video) {
      video.removeAttribute('autoplay');
      container.addEventListener('mouseenter', () => {
        video.play();
      });
      container.addEventListener('mouseleave', () => {
        video.pause();
        video.load();
      });
    }
  });
}
// Initialize on initial page load
document.addEventListener('DOMContentLoaded', () => {
  dd_video();
});
