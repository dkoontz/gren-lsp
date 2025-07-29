### import Foo.Bar as FB resolution
Check if aliasing a module works or if we need to account for that when doing the resolution of a symbol.


### Missing links
Only some types show links

### Missing tooltip
Stream usage of Task.andThen does not show tooltip. Open Task.elm module, now Task.andThen in Stream.elm shows tooltip.



## Types link to wrong definition

Can't repro?

One problem I noticed while testing is that sometimes when I click on a type that is linked I end up in the wrong place. I think perhaps we're not finding the one true definition of the type using the parse tree. If we go to where the symbol that the tooltip is displaying information for we should be able to examine the imports in that module to determine exactly where the type came from.