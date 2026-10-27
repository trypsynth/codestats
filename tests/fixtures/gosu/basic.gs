// expect: lang="Gosu" total=17 code=11 comment=2 blank=4 shebang=0
package example

uses java.util.ArrayList

// A simple Gosu class with a property.
class Greeter {
  var _names = new ArrayList<String>()

  property get Count() : int {
    return _names.size()
  }

  function add(name : String) {
    _names.add(name)
  }
}
