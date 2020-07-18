//! Trait and prelude for custom functions.

use crate::Pass;
use crate::syntax::ParseContext;
use crate::syntax::func::FuncHeader;
use crate::syntax::span::Spanned;

/// Types that are useful for creating your own functions.
pub mod prelude {
    pub use crate::{function, body, err};
    pub use crate::layout::prelude::*;
    pub use crate::layout::Command::{self, *};
    pub use crate::style::{LayoutStyle, PageStyle, TextStyle};
    pub use crate::syntax::SyntaxModel;
    pub use crate::syntax::expr::*;
    pub use crate::syntax::func::*;
    pub use crate::syntax::span::{Span, Spanned};
}


/// Parse a function from source code.
pub trait ParseFunc {
    /// A metadata type whose value is passed into the function parser. This
    /// allows a single function to do different things depending on the value
    /// that needs to be given when inserting the function into a
    /// [scope](crate::syntax::Scope).
    ///
    /// For example, the functions `word.spacing`, `line.spacing` and
    /// `par.spacing` are actually all the same function
    /// [`ContentSpacingFunc`](crate::library::ContentSpacingFunc) with the
    /// metadata specifiy which content should be spaced.
    type Meta: Clone;

    /// Parse the header and body into this function given a context.
    fn parse(
        header: FuncHeader,
        body: Option<Spanned<&str>>,
        ctx: ParseContext,
        metadata: Self::Meta,
    ) -> Pass<Self> where Self: Sized;
}

/// Allows to implement a function type concisely.
///
/// # Example
/// A function that hides its body depending on a boolean argument.
/// ```
/// use typstc::func::prelude::*;
///
/// function! {
///     #[derive(Debug, Clone, PartialEq)]
///     pub struct HiderFunc {
///         body: Option<SyntaxModel>,
///     }
///
///     parse(header, body, ctx, f) {
///         let body = body!(opt: body, ctx, f);
///         let hidden = header.args.pos.get::<bool>(&mut f.errors)
///             .or_missing(&mut f.errors, header.name.span, "hidden")
///             .unwrap_or(false);
///
///         HiderFunc { body: if hidden { None } else { body } }
///     }
///
///     layout(self, ctx, f) {
///         match &self.body {
///             Some(model) => vec![LayoutSyntaxModel(model)],
///             None => vec![],
///         }
///     }
/// }
/// ```
/// This function can be used as follows:
/// ```typst
/// [hider: true][Hi, you.]  => Nothing
/// [hider: false][Hi, you.] => Text: "Hi, you."
///
/// [hider][Hi, you.]        => Text: "Hi, you."
///  ^^^^^
///  missing argument: hidden
/// ```
///
/// # More examples
/// Look at the source code of the [`library`](crate::library) module for more
/// examples on how the macro works.
#[macro_export]
macro_rules! function {
    // Entry point.
    ($(#[$outer:meta])* $v:vis $storage:ident $name:ident $($r:tt)*) => {
        function!(@def($name) $(#[$outer])* $v $storage $name $($r)*);
    };
    (@def($name:ident) $definition:item $($r:tt)*) => {
        $definition
        function!(@meta($name) $($r)*);
    };

    // Metadata.
    (@meta($name:ident) type Meta = $meta:ty; $($r:tt)*) => {
        function!(@parse($name, $meta) $($r)*);
    };
    (@meta($name:ident) $($r:tt)*) => {
        function!(@parse($name, ()) $($r)*);
    };

    // Parse trait.
    (@parse($($a:tt)*) parse(default) $($r:tt)*) => {
        function!(@parse($($a)*) parse(_h, _b, _c, _f, _m) {Default::default() } $($r)*);
    };
    (@parse($($a:tt)*) parse($h:ident, $b:ident, $c:ident, $f:ident) $($r:tt)* ) => {
        function!(@parse($($a)*) parse($h, $b, $c, $f, _metadata) $($r)*);
    };
    (@parse($name:ident, $meta:ty) parse(
        $header:ident,
        $body:ident,
        $ctx:ident,
        $feedback:ident,
        $metadata:ident
    ) $code:block $($r:tt)*) => {
        impl $crate::func::ParseFunc for $name {
            type Meta = $meta;

            fn parse(
                #[allow(unused)] mut header: $crate::syntax::func::FuncHeader,
                #[allow(unused)] $body: Option<$crate::syntax::span::Spanned<&str>>,
                #[allow(unused)] $ctx: $crate::syntax::ParseContext,
                #[allow(unused)] $metadata: Self::Meta,
            ) -> $crate::Pass<Self> where Self: Sized {
                let mut feedback = $crate::Feedback::new();
                #[allow(unused)] let $header = &mut header;
                #[allow(unused)] let $feedback = &mut feedback;

                let func = $code;

                for arg in header.args.into_iter() {
                    feedback.errors.push(err!(arg.span; "unexpected argument"));
                }

                $crate::Pass::new(func, feedback)
            }
        }

        function!(@layout($name) $($r)*);
    };

    (@layout($name:ident) layout($this:ident, $ctx:ident, $feedback:ident) $code:block) => {
        impl $crate::syntax::Model for $name {
            fn layout<'a, 'b, 't>(
                #[allow(unused)] &'a $this,
                #[allow(unused)] mut $ctx: $crate::layout::LayoutContext<'b>,
            ) -> $crate::layout::DynFuture<'t, $crate::Pass<$crate::layout::Commands<'a>>>
            where
                'a: 't,
                'b: 't,
                Self: 't,
            {
                Box::pin(async move {
                    let mut feedback = $crate::Feedback::new();
                    #[allow(unused)] let $feedback = &mut feedback;
                    let commands = $code;
                    $crate::Pass::new(commands, feedback)
                })
            }
        }
    };
}

/// Parse the body of a function.
///
/// - If the function does not expect a body, use `body!(nope: body, feedback)`.
/// - If the function can have a body, use `body!(opt: body, ctx, feedback,
///   decos)`.
///
/// # Arguments
/// - The `$body` should be of type `Option<Spanned<&str>>`.
/// - The `$ctx` is the [`ParseContext`](crate::syntax::ParseContext) to use for
///   parsing.
/// - The `$feedback` should be a mutable references to a
///   [`Feedback`](crate::Feedback) struct which is filled with the feedback
///   information arising from parsing.
#[macro_export]
macro_rules! body {
    (opt: $body:expr, $ctx:expr, $feedback:expr) => ({
        $body.map(|body| {
            let parsed = $crate::syntax::parse(body.span.start, body.v, $ctx);
            $feedback.extend(parsed.feedback);
            parsed.output
        })
    });

    (nope: $body:expr, $feedback:expr) => {
        if let Some(body) = $body {
            $feedback.errors.push($crate::err!(body.span; "unexpected body"));
        }
    };
}