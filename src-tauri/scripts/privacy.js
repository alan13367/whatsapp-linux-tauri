(() => {
  const installStyle = () => {
    const styleId = "whatsapp-linux-privacy-style";
    if (document.getElementById(styleId)) return;

    const target = document.head || document.documentElement;
    if (!target) return;

    const style = document.createElement("style");
    style.id = styleId;
    style.textContent = `
      html.whatsapp-linux-privacy body {
        filter: blur(10px) !important;
        transition: filter 180ms ease-in-out !important;
      }
    `;
    target.appendChild(style);
  };

  installStyle();
  if (!document.documentElement) {
    document.addEventListener("DOMContentLoaded", installStyle, { once: true });
  }
})();
