/**
 * The following functions run and dynamically build the axe dashboard.
 * This dashboard will show dynamic result of the axe-core / cli being run on the page
 * with details on issues and reference links for more details.
 *
 * These scripts rely on the axe-core / cli to be loaded.
 * NPM: https://www.npmjs.com/package/@axe-core/cli
 * axe-core: https://github.com/dequelabs/axe-core
 * axe-cli: https://github.com/dequelabs/axe-cli
 * Rules & Tags: https://dequeuniversity.com/rules/worldspace/3.0/
 *
 */
function axerun() {
  // Only needed when loading content via ajax or HTMX in our case
  const axeElements = document.getElementsByClassName("dd-axe");
  Array.from(axeElements).forEach((element) => element.remove());
  // Main axe check script
  axe.run(
    {
      runOnly: {
        type: "tag",
        values: [
          "wcag2a",
          "wcag2aa",
          "ACT",
          "cat.aria",
          "cat.color",
          "cat.forms",
          "cat.keyboard",
          "cat.name-role-value",
          "cat.parsing",
          "cat.semantics",
          "cat.tables",
          "cat.text-alternatives",
        ],
      },
    },
    (err, results) => {
      // console.log(results);
      //console.log(err);
      createAccessibilityDashboard(results);
    },
  );
}
// Initial axe run after HTML loads
setTimeout(() => {
  axerun();
}, 1000);


// Escape any HTML chars before printing them out
function escapeHtml(str) {
  const htmlEntities = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  };
  return str.replace(/[&<>"']/g, (match) => htmlEntities[match]);
}

// Create the dashboard
function createAccessibilityDashboard(jsonData) {
  const ddaxe = document.createElement("div");
  ddaxe.className = "dd-axe";
  ddaxe.setAttribute("role", "complementary");
  ddaxe.innerHTML = `
      <div class="dd-axe__open-close dd-g -x-center -y-center"><i></i></div>
      <div class="dd-axe__content"><!-- dd-axe.js populates data here --></div>
  `;
  document.body.appendChild(ddaxe);

  const dashboardContainer = document.querySelector(".dd-axe__content");
  if (!dashboardContainer) {
    console.error("dd-axe not found on page. Check for missing element.");
    return;
  }

  // Calculate statistics
  const passesCount = jsonData.passes.reduce(
    (sum, item) => sum + item.nodes.length,
    0,
  );
  const violationsCount = jsonData.violations.reduce(
    (sum, item) => sum + item.nodes.length,
    0,
  );
  const totalItems = passesCount + violationsCount;
  const passPercentage =
    totalItems > 0 ? ((passesCount / totalItems) * 100).toFixed(2) : 0;
  const passPercentageRound = Math.round(passPercentage);
  var grade = "n/a";
  var gradeCSS = "";
  if (passPercentageRound >= 97) {
    grade = "A+";
    gradeCSS = "-grade-a-plus";
  } else if (passPercentageRound >= 93 && passPercentageRound <= 96) {
    grade = "A";
    gradeCSS = "-grade-a";
  } else if (passPercentageRound >= 90 && passPercentageRound <= 92) {
    grade = "A-";
    gradeCSS = "-grade-a-minus";
  } else if (passPercentageRound >= 87 && passPercentageRound <= 89) {
    grade = "B+";
    gradeCSS = "-grade-b-plus";
  } else if (passPercentageRound >= 83 && passPercentageRound <= 86) {
    grade = "B";
    gradeCSS = "-grade-b";
  } else if (passPercentageRound >= 80 && passPercentageRound <= 82) {
    grade = "B-";
    gradeCSS = "-grade-a-minus";
  } else if (passPercentageRound >= 77 && passPercentageRound <= 79) {
    grade = "C+";
    gradeCSS = "-grade-c-plus";
  } else if (passPercentageRound >= 73 && passPercentageRound <= 76) {
    grade = "C";
    gradeCSS = "-grade-c";
  } else if (passPercentageRound >= 70 && passPercentageRound <= 72) {
    grade = "C-";
    gradeCSS = "-grade-a-minus";
  } else if (passPercentageRound >= 67 && passPercentageRound <= 69) {
    grade = "D+";
    gradeCSS = "-grade-d-plus";
  } else if (passPercentageRound >= 63 && passPercentageRound <= 66) {
    grade = "D";
    gradeCSS = "-grade-d";
  } else if (passPercentageRound >= 60 && passPercentageRound <= 62) {
    grade = "D-";
    gradeCSS = "-grade-d-minus";
  } else if (passPercentageRound <= 59) {
    grade = "F";
    gradeCSS = "-grade-f";
  } else {
    grade = "n/a";
    gradeCSS = "-grade-na";
  }

  // Create statistics section
  const statsSection = document.createElement("div");
  statsSection.className = "dd-axe__stats";
  statsSection.innerHTML = `
      <div class="dd-g">
          <div class="dd-axe__stat-passed dd-u-12-24 l-box">
              <strong>Passed Items</strong>
              <p>${passesCount}</p>
          </div>
          <div class="dd-axe__stat-violations dd-u-12-24 l-box">
              <strong>Violations</strong>
              <p>${violationsCount}</p>
          </div>
          <div class="dd-axe__stat-passed-rate dd-u-12-24 l-box">
              <strong>Pass Rate</strong>
              <p>${passPercentage}%</p>
          </div>
          <div class="dd-axe__stat-grade ${gradeCSS} dd-u-12-24 l-box">
              <strong>Grade</strong>
              <p>${grade}</p>
          </div>
      </div>
  `;

  // Create violations section
  const violationsSection = document.createElement("div");
  violationsSection.className = "dd-axe__violations";

  const violationsTitle = document.createElement("h2");
  violationsTitle.textContent = "Violations Details";
  violationsSection.appendChild(violationsTitle);

  // Process each violation
  jsonData.violations.forEach((violation) => {
    violation.nodes.forEach((node) => {
      const violationItem = document.createElement("div");
      const impactClass = `-${node.impact || "unknown"}`;
      violationItem.className = `violation-item`;

      violationItem.innerHTML = `
        <div class="dd-alert ${impactClass}" role="alert">
          <div class="dd-alert__content dd-g">
            <div class="dd-u-1-1">
              <div class="l-box">
                <div class="dd-alert__sub-heading">
                  ${node.impact || "unknown"}
                </div>
              <div class="dd-alert__heading">
                  ${violation.description}
                </div>
                <div class="dd-alert__text">
                  <p><strong>Failure Summary:</strong> ${escapeHtml(node.failureSummary || "Not specified")}</p>
                  <p><strong>Target Element:</strong> <code>${escapeHtml(node.html)}</code></p>
                  <p><strong>Target Selector:</strong> ${escapeHtml(node.target.join(" > "))}</p></p>
                  <p><strong>Reference:</strong> <a href="${escapeHtml(violation.helpUrl)}" target="_blank">Learn more</a></p>
                </div>
              </div>
            </div>
          </div>
        </div>
      `;
      violationsSection.appendChild(violationItem);
      // Add impact class to the target element
      const targetElement = document.querySelector(node.target.join(' > '));
      if (targetElement) {
        targetElement.classList.add(`-${node.impact || 'unknown'}`);
      }
    });
  });

  // Clear existing content and append sections to dashboard
  dashboardContainer.innerHTML = "";
  dashboardContainer.appendChild(statsSection);
  dashboardContainer.appendChild(violationsSection);

  // Open and close the dashboard : AI!
  ddaxe.classList.add('-close'); // Start closed
  ddaxe.addEventListener('click', (event) => {
    // Check if the clicked element matches '.dd-axe__open-close'
    if (event.target.closest('.dd-axe__open-close')) {
      // Toggle class '-close' on the dashboard element
      ddaxe.classList.toggle('-close');
    }
  });
}

// Fire axe after HTMX settles
document.body.addEventListener("htmx:afterSettle", function (event) {
  axerun();
});
