import { vote_backend } from "../../declarations/vote_backend";

const getCurrentProposal = async (count) => {
  const getCurrentProposal = await vote_backend.get_proposal(Number(count));
  setCurrentProposal(getCurrentProposal);
}

document.querySelector("form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const button = e.target.querySelector("button");

  const name = document.getElementById("name").value.toString();

  button.setAttribute("disabled", true);

  // Interact with foo actor, calling the greet method
  const greeting = await vote_backend.greet(name);

  button.removeAttribute("disabled");

  document.getElementById("greeting").innerText = greeting;

  return false;
});
