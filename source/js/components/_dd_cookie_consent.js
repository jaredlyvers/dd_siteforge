function dd_cookie_consent() {
  // Check if consent is already stored
  const consent = localStorage.getItem('dd_cookie_consent');
  if (!consent) {
    // Show popup if no consent is stored
    const popup = document.getElementById('cookie-consent');
    popup.style.display = 'block';
  }
  else if (consent === 'accept') {
    // If consent is 'accept', add script tags
    addConsentScripts();
  }
}
function handleCookieConsent(choice) {
  // Save choice to localStorage
  localStorage.setItem('dd_cookie_consent', choice);
  // Close the popup
  const popup = document.getElementById('cookie-consent');
  popup.style.display = 'none';
  // If choice is 'accept', add script tags
  if (choice === 'accept') {
    addConsentScripts();
  }
}
function addConsentScripts() {
  // Add Google Tag Manager script for head
  const headScript = document.createElement('script');
  headScript.textContent = `(function(w,d,s,l,i){w[l]=w[l]||[];w[l].push({'gtm.start':
        new Date().getTime(),event:'gtm.js'});var f=d.getElementsByTagName(s)[0],
        j=d.createElement(s),dl=l!='dataLayer'?'&l='+l:'';j.async=true;j.src=
        'https://www.googletagmanager.com/gtm.js?id='+i+dl;f.parentNode.insertBefore(j,f);
      })(window,document,'script','dataLayer','GTM-T6QJCMC');`;
  document.head.appendChild(headScript);
  // Add Google Tag Manager noscript to body
  const noscript = document.createElement('noscript');
  noscript.innerHTML = `<iframe src="https://www.googletagmanager.com/ns.html?id=GTM-T6QJCMC"
    height="0" width="0" style="display:none;visibility:hidden"></iframe>`;
  document.body.insertBefore(noscript, document.body.firstChild);
}
// Initialize on page load
window.onload = dd_cookie_consent;
