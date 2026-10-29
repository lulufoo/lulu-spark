(function () {
  var actions = document.getElementById("actions");
  var note = document.getElementById("note");
  var allow = document.getElementById("allow");
  var deny = document.getElementById("deny");

  function showResult(text) {
    actions.hidden = true;
    note.hidden = false;
    note.textContent = text;
  }

  allow.addEventListener("click", function () {
    showResult("Allowed.");
  });
  deny.addEventListener("click", function () {
    showResult("Denied.");
  });
})();
