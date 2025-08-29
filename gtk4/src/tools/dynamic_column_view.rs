#![allow(dead_code)]
//! Adapted from Relm4 to allow for more dynamic columns

use gtk::{
    gio, glib,
    prelude::{Cast, CastNone, IsA, ListItemExt, ListModelExt, ObjectExt},
};
use relm4::gtk;
use relm4::typed_view::OrdFn;
use std::{
    any::Any,
    cell::{Ref, RefMut},
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display},
    marker::PhantomData,
};

mod selection_ext;
use selection_ext::RelmSelectionExt;
mod column_item;
use column_item::ColumnItem;

/// An item of a [`DynamicColumnView`].
pub trait RelmColumn: Any + Clone {
    /// The top-level widget for the list item.
    type Root: IsA<gtk::Widget>;

    /// The widgets created for the list item.
    type Widgets;

    /// Item whose data is shown in this column.
    type Item: Any;

    /// The columns created for this list item.
    fn column_name(&self) -> String;
    /// Whether to enable resizing for this column
    fn resize_enabled(&self) -> bool {
        false
    }
    /// Whether to enable automatic expanding for this column
    fn expand_enabled(&self) -> bool {
        false
    }

    /// Construct the widgets.
    fn setup(&self, list_item: &gtk::ListItem) -> (Self::Root, Self::Widgets);

    /// Bind the widgets to match the data of the list item.
    fn bind(&self, _item: &mut Self::Item, _widgets: &mut Self::Widgets, _root: &mut Self::Root) {}

    /// Undo the steps of [`RelmColumn::bind()`] if necessary.
    fn unbind(&self, _item: &mut Self::Item, _widgets: &mut Self::Widgets, _root: &mut Self::Root) {
    }

    /// Undo the steps of [`RelmColumn::setup()`] if necessary.
    fn teardown(&self, _list_item: &gtk::ListItem) {}

    /// Sorter for column.
    #[must_use]
    fn sort_fn(&self) -> OrdFn<Self::Item> {
        None
    }
}

/// Simplified trait for creating columns with only one `gtk::Label` widget per-entry (i.e. a text cell)
pub trait LabelColumn: 'static + Clone {
    /// Item of the model
    type Item: Any;
    /// Value of the column
    type Value: PartialOrd + Display;

    /// Name of the column
    fn column_name(&self) -> String;
    /// Whether to enable the sorting for this column
    fn sort_enabled(&self) -> bool {
        false
    }
    /// Whether to enable resizing for this column
    fn resize_enabled(&self) -> bool {
        false
    }
    /// Whether to enable automatic expanding for this column
    fn expand_enabled(&self) -> bool {
        false
    }

    /// Get the value that this column represents.
    fn get_cell_value(&self, item: &Self::Item) -> Self::Value;
    /// Format the value for presentation in the text cell.
    fn format_cell_value(&self, value: &Self::Value) -> String {
        value.to_string()
    }
}

impl<C> RelmColumn for C
where
    C: LabelColumn,
{
    type Root = gtk::Label;
    type Widgets = ();
    type Item = C::Item;

    fn column_name(&self) -> String {
        <Self as LabelColumn>::column_name(self)
    }

    fn resize_enabled(&self) -> bool {
        <Self as LabelColumn>::resize_enabled(self)
    }

    fn expand_enabled(&self) -> bool {
        <Self as LabelColumn>::expand_enabled(self)
    }

    fn setup(&self, _: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Label::new(None), ())
    }

    fn bind(&self, item: &mut Self::Item, _: &mut Self::Widgets, label: &mut Self::Root) {
        label.set_label(&C::format_cell_value(self, &C::get_cell_value(self, item)));
    }

    fn sort_fn(&self) -> OrdFn<Self::Item> {
        if self.sort_enabled() {
            let copy = self.clone();
            Some(Box::new(move |a, b| {
                let a = C::get_cell_value(&copy, a);
                let b = C::get_cell_value(&copy, b);
                a.partial_cmp(&b).unwrap_or(Ordering::Equal)
            }))
        } else {
            None
        }
    }
}

/// A high-level wrapper around [`gio::ListStore`],
/// [`gtk::SignalListItemFactory`] and [`gtk::ColumnView`].
///
/// [`TypedColumnView`] aims at keeping nearly the same functionality and
/// flexibility of the raw bindings while introducing a more idiomatic
/// and type-safe interface.
pub struct DynamicColumnView<T, S> {
    /// The internal list view.
    pub view: gtk::ColumnView,
    /// The internal selection model.
    pub selection_model: S,
    columns: HashMap<String, gtk::ColumnViewColumn>,
    store: gio::ListStore,
    active_model: gio::ListModel,
    base_model: gio::ListModel,
    _ty: PhantomData<*const T>,
}

impl<T, S> Debug for DynamicColumnView<T, S>
where
    T: Debug,
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedColumnView")
            .field("store", &self.store)
            .field("view", &self.view)
            .field("active_model", &self.active_model)
            .field("base_model", &self.base_model)
            .field("selection_model", &self.selection_model)
            .finish()
    }
}

impl<T, S> Default for DynamicColumnView<T, S>
where
    T: Any,
    S: RelmSelectionExt,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, S> DynamicColumnView<T, S>
where
    T: Any,
    S: RelmSelectionExt,
{
    /// Create a new, empty [`TypedColumnView`].
    #[must_use]
    pub fn new() -> Self {
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();

        let model: gio::ListModel = store.clone().upcast();

        let b = gtk::SortListModel::new(Some(model), None::<gtk::Sorter>);

        let base_model: gio::ListModel = b.clone().upcast();

        let selection_model = S::new_model(base_model.clone());
        let view = gtk::ColumnView::new(Some(selection_model.clone()));
        b.set_sorter(view.sorter().as_ref());

        Self {
            store,
            view,
            columns: HashMap::new(),
            active_model: base_model.clone(),
            base_model,
            _ty: PhantomData,
            selection_model,
        }
    }

    /// Append column to this typed view
    pub fn append_column<C>(&mut self, column: C)
    where
        C: RelmColumn<Item = T>,
    {
        let factory = gtk::SignalListItemFactory::new();
        let c = column.clone();
        factory.connect_setup(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");

            let (root, widgets) = c.setup(list_item);
            unsafe { root.set_data("widgets", widgets) };
            list_item.set_child(Some(&root));
        });

        #[inline]
        fn modify_widgets<T, C>(
            list_item: &glib::Object,
            f: impl FnOnce(&mut T, &mut C::Widgets, &mut C::Root),
        ) where
            T: Any,
            C: RelmColumn<Item = T>,
        {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");

            let widget = list_item.child();

            let obj = list_item.item().unwrap();
            let mut obj = get_mut_value::<T>(&obj);

            let mut root = widget.and_downcast::<C::Root>().unwrap();

            let mut widgets = unsafe { root.steal_data("widgets") }.unwrap();
            (f)(&mut *obj, &mut widgets, &mut root);
            unsafe { root.set_data("widgets", widgets) };
        }

        let c = column.clone();
        factory.connect_bind(move |_, list_item| {
            modify_widgets::<T, C>(list_item.upcast_ref(), |obj, widgets, root| {
                c.bind(obj, widgets, root);
            });
        });

        let c = column.clone();
        factory.connect_unbind(move |_, list_item| {
            modify_widgets::<T, C>(list_item.upcast_ref(), |obj, widgets, root| {
                c.unbind(obj, widgets, root);
            });
        });

        let c = column.clone();
        factory.connect_teardown(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");

            c.teardown(list_item);
        });

        let sort_fn = column.sort_fn();

        let c = gtk::ColumnViewColumn::new(Some(&column.column_name()), Some(factory));
        c.set_resizable(column.resize_enabled());
        c.set_expand(column.expand_enabled());

        if let Some(sort_fn) = sort_fn {
            c.set_sorter(Some(&gtk::CustomSorter::new(move |first, second| {
                let first = get_value::<T>(first);
                let second = get_value::<T>(second);

                sort_fn(&first, &second).into()
            })))
        }

        self.view.append_column(&c);
        self.columns.insert(column.column_name(), c);
    }

    /// Remove all columns.
    pub fn clear_columns(&mut self) {
        for (_column_name, column) in &self.columns {
            self.view.remove_column(column);
        }
        self.columns.clear();
    }

    /// Get columns currently associated with this view.
    pub fn get_columns(&self) -> &HashMap<String, gtk::ColumnViewColumn> {
        &self.columns
    }

    /// Add a new item at the end of the list.
    pub fn append(&mut self, value: T) {
        self.store.append(&glib::BoxedAnyObject::new(value));
    }

    /// Add new items from an iterator the the end of the list.
    pub fn extend_from_iter<I: IntoIterator<Item = T>>(&mut self, init: I) {
        let objects: Vec<glib::BoxedAnyObject> =
            init.into_iter().map(glib::BoxedAnyObject::new).collect();
        self.store.extend_from_slice(&objects);
    }

    /// Returns true if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the length of the list (without filters).
    pub fn len(&self) -> u32 {
        self.store.n_items()
    }

    /// Get the [`TypedListItem`] at the specified position.
    ///
    /// Returns [`None`] if the position is invalid.
    pub fn get(&self, position: u32) -> Option<ColumnItem<T>> {
        if let Some(obj) = self.store.item(position) {
            let wrapper = obj.downcast::<glib::BoxedAnyObject>().unwrap();
            Some(ColumnItem::new(wrapper))
        } else {
            None
        }
    }

    /// Get the visible [`TypedListItem`] at the specified position,
    /// (the item at the given position after filtering and sorting).
    ///
    /// Returns [`None`] if the position is invalid.
    pub fn get_visible(&self, position: u32) -> Option<ColumnItem<T>> {
        if let Some(obj) = self.active_model.item(position) {
            let wrapper = obj.downcast::<glib::BoxedAnyObject>().unwrap();
            Some(ColumnItem::new(wrapper))
        } else {
            None
        }
    }

    /// Insert an item at a specific position.
    pub fn insert(&mut self, position: u32, value: T) {
        self.store
            .insert(position, &glib::BoxedAnyObject::new(value));
    }

    /// Insert an item into the list and calculate its position from
    /// a sorting function.
    pub fn insert_sorted<F>(&self, value: T, mut compare_func: F) -> u32
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let item = glib::BoxedAnyObject::new(value);

        let compare = move |first: &glib::Object, second: &glib::Object| -> Ordering {
            let first = get_value::<T>(first);
            let second = get_value::<T>(second);
            compare_func(&first, &second)
        };

        self.store.insert_sorted(&item, compare)
    }

    /// Remove an item at a specific position.
    pub fn remove(&mut self, position: u32) {
        self.store.remove(position);
    }

    /// Remove all items.
    pub fn clear(&mut self) {
        self.store.remove_all();
    }
}

fn get_value<T: 'static>(obj: &glib::Object) -> Ref<'_, T> {
    let wrapper = obj.downcast_ref::<glib::BoxedAnyObject>().unwrap();
    wrapper.borrow()
}

fn get_mut_value<T: 'static>(obj: &glib::Object) -> RefMut<'_, T> {
    let wrapper = obj.downcast_ref::<glib::BoxedAnyObject>().unwrap();
    wrapper.borrow_mut()
}
