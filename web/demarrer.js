// Démarre le client si le navigateur propose WebGPU (seule API de rendu, F-04).
const message = document.getElementById("message");
if (!("gpu" in navigator)) {
  message.textContent = "Ce navigateur ne propose pas WebGPU, nécessaire au jeu.";
  message.style.display = "grid";
} else {
  const { default: init } = await import("./beta1_client.js");
  await init();
}
