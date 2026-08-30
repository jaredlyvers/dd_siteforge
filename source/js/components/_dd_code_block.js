// function that will escape all HTML chars from the passed element and add the output to the proper code element: AI!
function escape_code_block() {
  // Find all <code> elements
  document.querySelectorAll('code').forEach(codeBlock => {
    // Skip if already processed (prevents double-wrapping)
    if (codeBlock.dataset.enhanced) return;
    //codeBlock.dataset.enhanced = 'true';

    // Get the original raw content (before we escape it)
    const originalContent = codeBlock.innerHTML;

    // Escape it so it displays correctly as text
    const escaped = originalContent
      .replace(/&/g,  '&amp;')
      .replace(/</g,  '&lt;')
      .replace(/>/g,  '&gt;')
      .replace(/"/g,  '&quot;')
      .replace(/'/g,  '&#039;');

    // Create copy button
    const copyBtn = document.createElement('button');
    copyBtn.textContent = 'Copy';
    copyBtn.style.position = 'absolute';
    copyBtn.style.top = '6px';
    copyBtn.style.right = '8px';
    copyBtn.classList.add("-ghost");
    copyBtn.style.zIndex = '10';

    // Copy functionality
    copyBtn.addEventListener('click', () => {
      navigator.clipboard.writeText(originalContent)
        .then(() => {
          const originalText = copyBtn.textContent;
          copyBtn.textContent = 'Copied!';
          copyBtn.style.backgroundColor = '#d4edda';
          setTimeout(() => {
            copyBtn.textContent = originalText;
            copyBtn.style.backgroundColor = '#f0f0f0';
          }, 1800);
        })
        .catch(err => {
          console.error('Copy failed:', err);
          copyBtn.textContent = 'Copy failed';
        });
    });

    // Replace content with escaped version
    codeBlock.innerHTML = escaped;

    // Add copy button to code block
    codeBlock.appendChild(copyBtn);
  });
}
