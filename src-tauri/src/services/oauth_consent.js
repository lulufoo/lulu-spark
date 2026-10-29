(function () {
  var allow = document.getElementById("allow");
  var deny = document.getElementById("deny");
  var query = new URLSearchParams(location.search);

  function submit(decision) {
    var form = document.createElement("form");
    form.method = "POST";
    form.action = "/oauth/consent";
    function add(name, value) {
      var input = document.createElement("input");
      input.type = "hidden";
      input.name = name;
      input.value = value;
      form.appendChild(input);
    }
    add("decision", decision);
    add("redirect_uri", query.get("redirect_uri") || "");
    add("client_id", query.get("client_id") || "");
    add("code_challenge", query.get("code_challenge") || "");
    add("state", query.get("state") || "");
    document.body.appendChild(form);
    form.submit();
  }

  allow.addEventListener("click", function () {
    submit("allow");
  });
  deny.addEventListener("click", function () {
    submit("deny");
  });
})();
