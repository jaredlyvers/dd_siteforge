// This function is called from the dd_animate.js file
function dd_milestones_count() {
  document.querySelectorAll('.dd-milestones .dd-milestones__percentage').forEach(function(elem) {
    var milestone_start = 0;
    var milestone_end = elem.dataset.number;
    var milestone_elem = elem.querySelector('.number');
    var milestone_animate = setInterval(function() {
      if (milestone_start <= milestone_end) {
        milestone_elem.innerText = milestone_start;
        milestone_start++;
      } else {
        clearInterval(milestone_animate);
      }
    }, 60);
  });
}
// Initialize on initial page load
document.addEventListener('DOMContentLoaded', () => {
  dd_milestones_count();
});
dd_milestones_count();
// Fire axe after HTMX settles
document.body.addEventListener("htmx:afterSettle", function (event) {
  dd_milestones_count();
});
