import collomatique

# The rust side stands in for the application, and answers with the document it
# holds at the moment of the call. `monday` is a monday.
first = collomatique.current_document()
second = collomatique.current_document()

# The console is not modal: between two calls the user may have edited the
# application's document, so each call is a copy of its own rather than the one
# copy a script is handed.
assert first is not second

start = first.periods.first_week
first.periods.set_first_week(monday)
assert second.periods.first_week == start

collomatique.send_to_host(first)

# The application named what it now holds when it took the first one, so this
# second send speaks of that document and not of the one that was read.
collomatique.send_to_host(first)

# A document the application never handed over has no name of its own.
collomatique.send_to_host(collomatique.new_document())
