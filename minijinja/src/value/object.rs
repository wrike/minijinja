use std::any::Any;
use std::fmt;
use std::ops::Range;

use crate::error::{Error, ErrorKind};
use crate::value::Value;
use crate::vm::State;

/// A utility trait that represents a dynamic object.
///
/// The engine uses the [`Value`] type to represent values that the engine
/// knows about.  Most of these values are primitives such as integers, strings
/// or maps.  However it is also possible to expose custom types without
/// undergoing a serialization step to the engine.  For this to work a type
/// needs to implement the [`Object`] trait and be wrapped in a value with
/// [`Value::from_object`](crate::value::Value::from_object). The ownership of
/// the object will then move into the value type.
//
/// The engine uses reference counted objects with interior mutability in the
/// value type.  This means that all trait methods take `&self` and types like
/// [`Mutex`](std::sync::Mutex) need to be used to enable mutability.
//
/// Objects need to implement [`Display`](std::fmt::Display) which is used by
/// the engine to convert the object into a string if needed.  Additionally
/// [`Debug`](std::fmt::Debug) is required as well.
///
/// The exact runtime characteristics of the object are influenced by the
/// [`kind`](Self::kind) of the object.  By default an object can just be
/// stringified and methods can be called.
///
/// For examples of how to implement objects refer to [`SeqObject`] and
/// [`StructObject`].
pub trait Object: fmt::Display + fmt::Debug + Any + Sync + Send {
    /// Describes the kind of an object.
    ///
    /// If not implemented behavior for an object is [`ObjectKind::Plain`]
    /// which just means that it's stringifyable and potentially can be
    /// called or has methods.
    ///
    /// For more information see [`ObjectKind`].
    fn kind(&self) -> ObjectKind<'_> {
        ObjectKind::Plain
    }

    /// Called when the engine tries to call a method on the object.
    ///
    /// It's the responsibility of the implementer to ensure that an
    /// error is generated if an invalid method is invoked.
    ///
    /// To convert the arguments into arguments use the
    /// [`from_args`](crate::value::from_args) function.
    fn call_method(&self, state: &State, name: &str, args: &[Value]) -> Result<Value, Error> {
        let _state = state;
        let _args = args;
        Err(Error::new(
            ErrorKind::UnknownMethod,
            format!("object has no method named {}", name),
        ))
    }

    /// Called when the object is invoked directly.
    ///
    /// The default implementation just generates an error that the object
    /// cannot be invoked.
    ///
    /// To convert the arguments into arguments use the
    /// [`from_args`](crate::value::from_args) function.
    fn call(&self, state: &State, args: &[Value]) -> Result<Value, Error> {
        let _state = state;
        let _args = args;
        Err(Error::new(
            ErrorKind::InvalidOperation,
            "tried to call non callable object",
        ))
    }
}

impl<T: Object> Object for std::sync::Arc<T> {
    #[inline]
    fn kind(&self) -> ObjectKind<'_> {
        T::kind(self)
    }

    #[inline]
    fn call_method(&self, state: &State, name: &str, args: &[Value]) -> Result<Value, Error> {
        T::call_method(self, state, name, args)
    }

    #[inline]
    fn call(&self, state: &State, args: &[Value]) -> Result<Value, Error> {
        T::call(self, state, args)
    }
}

/// A kind defines the object's behavior.
///
/// When a dynamic [`Object`] is implemented, it can be of one of the kinds
/// here.  The default behavior will be a [`Plain`](Self::Plain) object which
/// doesn't do much other than that it can be printed.  For an object to turn
/// into a [struct](Self::Struct) or [sequence](Self::Seq) the necessary kind
/// has to be returned with a pointer to itself.
///
/// Today object's can have the behavior of structs and sequences but this
/// might expand in the future.  It does mean that not all types of values can
/// be represented by objects.
#[non_exhaustive]
pub enum ObjectKind<'a> {
    /// This object is a plain object.
    ///
    /// Such an object has no attributes but it might be callable and it
    /// can be stringified.  When serialized it's serialized in it's
    /// stringified form.
    Plain,

    /// This object is a sequence.
    ///
    /// Requires that the object implements [`SeqObject`].
    Seq(&'a dyn SeqObject),

    /// This object is a struct (map with string keys).
    ///
    /// Requires that the object implements [`StructObject`].
    Struct(&'a dyn StructObject),
}

/// Provides the behavior of an [`Object`] holding sequence of values.
///
/// An object holding a sequence of values (tuple, list etc.) can be
/// represented by this trait.
///
/// # Simplified Example
///
/// For sequences which do not need any special method behavior, the [`Value`]
/// type is capable of automatically constructing a wrapper [`Object`] by using
/// [`Value::from_seq_object`].  In that case only [`SeqObject`] needs to be
/// implemented and the value will provide default implementations for
/// stringification and debug printing.
///
/// ```
/// use minijinja::value::{Value, SeqObject};
///
/// struct Point(f32, f32, f32);
///
/// impl SeqObject for Point {
///     fn get_item(&self, idx: usize) -> Option<Value> {
///         match idx {
///             0 => Some(Value::from(self.0)),
///             1 => Some(Value::from(self.1)),
///             2 => Some(Value::from(self.2)),
///             _ => None,
///         }
///     }
///
///     fn item_count(&self) -> usize {
///         3
///     }
/// }
///
/// let value = Value::from_seq_object(Point(1.0, 2.5, 3.0));
/// ```
///
/// # Full Example
///
/// This example shows how one can use [`SeqObject`] in conjunction
/// with a fully customized [`Object`].  Note that in this case not
/// only [`Object`] needs to be implemented, but also [`Debug`] and
/// [`Display`](std::fmt::Display) no longer come for free.
///
/// ```
/// use std::fmt;
/// use minijinja::value::{Value, Object, ObjectKind, SeqObject};
///
/// #[derive(Debug, Clone)]
/// struct Point(f32, f32, f32);
///
/// impl fmt::Display for Point {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "({}, {}, {})", self.0, self.1, self.2)
///     }
/// }
///
/// impl Object for Point {
///     fn kind(&self) -> ObjectKind<'_> {
///         ObjectKind::Seq(self)
///     }
/// }
///
/// impl SeqObject for Point {
///     fn get_item(&self, idx: usize) -> Option<Value> {
///         match idx {
///             0 => Some(Value::from(self.0)),
///             1 => Some(Value::from(self.1)),
///             2 => Some(Value::from(self.2)),
///             _ => None,
///         }
///     }
///
///     fn item_count(&self) -> usize {
///         3
///     }
/// }
///
/// let value = Value::from_object(Point(1.0, 2.5, 3.0));
/// ```
pub trait SeqObject: Send + Sync {
    /// Looks up an item by index.
    ///
    /// Sequences should provide a value for all items in the range of `0..item_count`
    /// but the engine will assume that items within the range are `Undefined`
    /// if `None` is returned.
    fn get_item(&self, idx: usize) -> Option<Value>;

    /// Returns the number of items in the sequence.
    fn item_count(&self) -> usize;
}

impl dyn SeqObject + '_ {
    /// Convenient iterator over a [`SeqObject`].
    pub fn iter(&self) -> SeqObjectIter<'_> {
        SeqObjectIter {
            seq: self,
            range: 0..self.item_count(),
        }
    }
}

impl<T: SeqObject> SeqObject for std::sync::Arc<T> {
    #[inline]
    fn get_item(&self, idx: usize) -> Option<Value> {
        T::get_item(self, idx)
    }

    #[inline]
    fn item_count(&self) -> usize {
        T::item_count(self)
    }
}

impl<'a, T: Into<Value> + Send + Sync + Clone> SeqObject for &'a [T] {
    #[inline(always)]
    fn get_item(&self, idx: usize) -> Option<Value> {
        self.get(idx).cloned().map(Into::into)
    }

    #[inline(always)]
    fn item_count(&self) -> usize {
        self.len()
    }
}

impl SeqObject for Vec<Value> {
    #[inline(always)]
    fn get_item(&self, idx: usize) -> Option<Value> {
        self.get(idx).cloned().map(Into::into)
    }

    #[inline(always)]
    fn item_count(&self) -> usize {
        self.len()
    }
}

/// Iterates over [`SeqObject`]
pub struct SeqObjectIter<'a> {
    seq: &'a dyn SeqObject,
    range: Range<usize>,
}

impl<'a> Iterator for SeqObjectIter<'a> {
    type Item = Value;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.range
            .next()
            .map(|idx| self.seq.get_item(idx).unwrap_or(Value::UNDEFINED))
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<'a> DoubleEndedIterator for SeqObjectIter<'a> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range
            .next_back()
            .map(|idx| self.seq.get_item(idx).unwrap_or(Value::UNDEFINED))
    }
}

impl<'a> ExactSizeIterator for SeqObjectIter<'a> {}

/// Provides the behavior of an [`Object`] holding a struct.
///
/// An basic object with the shape and behavior of a struct (that means a
/// map with string keys) can be represented by this trait.
///
/// # Simplified Example
///
/// For structs which do not need any special method behavior or methods, the
/// [`Value`] type is capable of automatically constructing a wrapper [`Object`]
/// by using [`Value::from_struct_object`].  In that case only [`StructObject`]
/// needs to be implemented and the value will provide default implementations
/// for stringification and debug printing.
///
/// ```
/// use minijinja::value::{Value, StructObject};
///
/// struct Point(f32, f32, f32);
///
/// impl StructObject for Point {
///     fn get_field(&self, name: &str) -> Option<Value> {
///         match name {
///             "x" => Some(Value::from(self.0)),
///             "y" => Some(Value::from(self.1)),
///             "z" => Some(Value::from(self.2)),
///             _ => None,
///         }
///     }
///
///     fn fields(&self) -> Box<dyn Iterator<Item = &str> + '_> {
///         Box::new(["x", "y", "z"].into_iter())
///     }
/// }
///
/// let value = Value::from_struct_object(Point(1.0, 2.5, 3.0));
/// ```
///
/// # Full Example
///
/// The following example shows how to implement a dynamic object which
/// represents a struct.  Note that in this case not only [`Object`] needs to be
/// implemented, but also [`Debug`] and [`Display`](std::fmt::Display) no longer
/// come for free.
///
/// ```
/// use std::fmt;
/// use minijinja::value::{Value, Object, ObjectKind, StructObject};
///
/// #[derive(Debug, Clone)]
/// struct Point(f32, f32, f32);
///
/// impl fmt::Display for Point {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "({}, {}, {})", self.0, self.1, self.2)
///     }
/// }
///
/// impl Object for Point {
///     fn kind(&self) -> ObjectKind<'_> {
///         ObjectKind::Struct(self)
///     }
/// }
///
/// impl StructObject for Point {
///     fn get_field(&self, name: &str) -> Option<Value> {
///         match name {
///             "x" => Some(Value::from(self.0)),
///             "y" => Some(Value::from(self.1)),
///             "z" => Some(Value::from(self.2)),
///             _ => None,
///         }
///     }
///
///     fn fields(&self) -> Box<dyn Iterator<Item = &str> + '_> {
///         Box::new(["x", "y", "z"].into_iter())
///     }
/// }
///
/// let value = Value::from_object(Point(1.0, 2.5, 3.0));
/// ```
pub trait StructObject: Send + Sync {
    /// Invoked by the engine to get a field of a struct.
    ///
    /// Where possible it's a good idea for this to align with the return value
    /// of [`fields`](Self::fields) but it's not necessary.
    ///
    /// If an field does not exist, `None` shall be returned.
    ///
    /// A note should be made here on side effects: unlike calling objects or
    /// calling methods on objects, accessing fields is not supposed to
    /// have side effects.  Neither does this API get access to the interpreter
    /// [`State`] nor is there a channel to send out failures as only an option
    /// can be returned.  If you do plan on doing something in field access
    /// that is fallible, instead use a method call.
    fn get_field(&self, name: &str) -> Option<Value>;

    /// Iterates over the fields.
    ///
    /// The default implementation returns an empty iterator.
    fn fields(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        Box::new(None.into_iter())
    }

    /// Returns the number of fields in the struct.
    ///
    /// The default implementation returns the number of fields.
    fn field_count(&self) -> usize {
        self.fields().count()
    }
}

impl<T: StructObject> StructObject for std::sync::Arc<T> {
    #[inline]
    fn get_field(&self, name: &str) -> Option<Value> {
        T::get_field(self, name)
    }

    #[inline]
    fn fields(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        T::fields(self)
    }

    #[inline]
    fn field_count(&self) -> usize {
        T::field_count(self)
    }
}

#[repr(transparent)]
pub struct SimpleSeqObject<T>(pub T);

impl<T: SeqObject + 'static> fmt::Display for SimpleSeqObject<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ok!(write!(f, "["));
        for (idx, val) in (&self.0 as &dyn SeqObject).iter().enumerate() {
            if idx > 0 {
                ok!(write!(f, ", "));
            }
            ok!(write!(f, "{:?}", val));
        }
        write!(f, "]")
    }
}

impl<T: SeqObject + 'static> fmt::Debug for SimpleSeqObject<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries((&self.0 as &dyn SeqObject).iter())
            .finish()
    }
}

impl<T: SeqObject + 'static> Object for SimpleSeqObject<T> {
    fn kind(&self) -> ObjectKind<'_> {
        ObjectKind::Seq(&self.0)
    }
}

#[repr(transparent)]
pub struct SimpleStructObject<T>(pub T);

impl<T: StructObject + 'static> fmt::Display for SimpleStructObject<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ok!(write!(f, "["));
        for (idx, field) in self.0.fields().enumerate() {
            if idx > 0 {
                ok!(write!(f, ", "));
            }
            let val = self.0.get_field(field).unwrap_or(Value::UNDEFINED);
            ok!(write!(f, "{:?}: {:?}", field, val));
        }
        write!(f, "]")
    }
}

impl<T: StructObject + 'static> fmt::Debug for SimpleStructObject<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut m = f.debug_map();
        for field in self.0.fields() {
            let value = self.0.get_field(field).unwrap_or(Value::UNDEFINED);
            m.entry(&field, &value);
        }
        m.finish()
    }
}

impl<T: StructObject + 'static> Object for SimpleStructObject<T> {
    fn kind(&self) -> ObjectKind<'_> {
        ObjectKind::Struct(&self.0)
    }
}
