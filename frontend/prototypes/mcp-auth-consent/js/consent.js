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
    showResult("Allowed. Design draft only — nothing was sent.");
  });
  deny.addEventListener("click", function () {
    showResult("Denied. Design draft only — nothing was sent.");
  });
})();
