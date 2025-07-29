### Type constructors don't reference their type
For type constructors, they currently show like this: `Nothing constructor` as compared to Maybe which shows like this: `Maybe type`. I would really like a type constructor to show the type it constructs. So Nothing would show `Nothing constructor for Maybe type`.


### Tooltip uses "module" instead of "type"
```gren
type Error
    = Closed
    | Cancelled String
    | Locked
```
creates tooltip

Cancelled constructor
from module Error


### Tooltip links
Links shows as unformatted markdown

### Missing links
Only some types show links

### Missing tooltip
Task shows tooltip, `andThen` does not