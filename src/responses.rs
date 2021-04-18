use nom::{
    bytes::complete::{tag, take_till1, take_while_m_n},
    combinator::map,
    sequence::separated_pair,
    IResult,
};

macro_rules! parser {
    ( $name:ident { $( $items:tt )* } ) => {
        parser!( @structdef $name ( $( $items )* ) => () );

        impl $name {
            pub fn parse_from(s: &str) -> ::nom::IResult<&str, $name> {
                parser!( @parse $name s: () <= $( $items )* )
            }
        }
    };

    ( @structdef $name:ident () => ( $( $member:ident: $mty:ty, )* ) ) => {
        #[derive(Debug, PartialEq)]
        pub struct $name {
            $( pub $member: $mty, )*
        }
    };

    ( @structdef $sname:ident ( $_:literal $( $rest:tt )* )
      => ( $( $member:ident: $mty:ty, )* ) ) =>
    {
        parser!( @structdef $sname ( $($rest)* )
            => ( $( $member: $mty, )* ) );
    };

    ( @structdef $sname:ident ( { $name:ident: $ty:ty } $( $rest:tt )* )
      => ( $( $member:ident: $mty:ty, )* ) ) =>
    {
        parser!( @structdef $sname ( $($rest)* )
            => ( $name: $ty, $( $member: $mty, )* ) );
    };

    ( @parse $sname:ident $s:ident: ($($r:ident)*) <= $tag:literal $( $rest:tt )* ) => {{
        let (s, _) = ::nom::bytes::complete::tag($tag)($s)?;
        parser!( @parse $sname s: ($($r)*) <= $( $rest )* )
    }};

    ( @parse $sname:ident $s:ident: ($($r:ident)*) <= { $name:ident: String } $tag:literal $( $rest:tt )* ) => {{
        let (s, $name) = ::nom::bytes::complete::take_until($tag)($s)?;
        let (s, _) = ::nom::bytes::complete::tag($tag)(s)?;
        let $name = $name.to_owned();
        parser!( @parse $sname s: ($($r)* $name) <= $( $rest )* )
    }};

    ( @parse $sname:ident $s:ident: ($($r:ident)*) <= { $name:ident: i64 } $( $rest:tt )* ) => {{
        let (s, $name) = ::nom::bytes::complete::take_while1(|c: char| c.is_ascii_digit() || c == '-')($s)?;
        let $name: i64 = $name.parse().unwrap();
        parser!( @parse $sname s: ($($r)* $name) <= $( $rest )* )
    }};

    ( @parse $sname:ident $s:ident: ($($r:ident)*) <= { $name:ident: i32 } $( $rest:tt )* ) => {{
        let (s, $name): (_, i32) = if let Ok((s, _none)) = ::nom::bytes::complete::tag::<_, _, ()>("none")($s) {
            (s, 0)
        }
        else {
            let (s, val) = ::nom::bytes::complete::take_while1(|c: char| c.is_ascii_digit() || c == '-')($s)?;
            (s, val.parse().unwrap())
        };
        parser!( @parse $sname s: ($($r)* $name) <= $( $rest )* )
    }};

    ( @parse $sname:ident $s:ident: ($($r:ident)*) <= { $name:ident: i16 } $( $rest:tt )* ) => {{
        let (s, $name) = ::nom::bytes::complete::take_while1(|c: char| c.is_ascii_digit() || c == '-')($s)?;
        let $name: i16 = $name.parse().unwrap();
        parser!( @parse $sname s: ($($r)* $name) <= $( $rest )* )
    }};

    ( @parse $sname:ident $s:ident: ($($r:ident)*) <= { $name:ident: u32 } $( $rest:tt )* ) => {{
        let (s, $name) = ::nom::bytes::complete::take_while1(|c: char| c.is_ascii_digit() || c == '-')($s)?;
        let $name: u32 = $name.parse().unwrap();
        parser!( @parse $sname s: ($($r)* $name) <= $( $rest )* )
    }};

    ( @parse $sname:ident $s:ident: ($($r:ident)*) <= { $name:ident: bool } $( $rest:tt )* ) => {{
        use ::nom::bytes::complete::tag;
        use ::nom::combinator::map;
        let (s, $name) = ::nom::branch::alt((
                map(tag("0"), |_| false),
                map(tag("1"), |_| true),
        ))($s)?;
        parser!( @parse $sname s: ($($r)* $name) <= $( $rest )* )
    }};

    ( @parse $sname:ident $s:ident: ($($r:ident)*) <= ) => {{
        let _ = ::nom::combinator::eof($s)?;
        #[allow(clippy::inconsistent_struct_constructor)]
        Ok(($s, $sname {
            $( $r ),*
        }))
    }};
}

macro_rules! match_res {
    ( match $s:expr; $name:ident { $( $member:ident ),* $(,)? } => $block:block, $($rest:tt)* ) => {
        let s = $s;
        if let Ok((_, $name { $( $member ),* })) = $name::parse_from(&s) {
            $block
        } else {
            match_res!( match s; $($rest)* );
        }
    };

    ( match $s:expr; return $name:ident, $($rest:tt)* ) => {
        let s = $s;
        if let Ok((_, msg)) = $name::parse_from(&s) {
            return Ok(msg)
        } else {
            match_res!( match s; $($rest)* );
        }
    };

    ( match $s:expr; Err($e:ident) => $block:block ) => {
        let $e = crate::responses::Error::parse_from(&$s);
        $block;
    };

    ( match $s:expr; ) => {
        let e = crate::responses::Error::parse_from(&$s);
        return Err(e.into());
    };
}

#[derive(Debug, PartialEq)]
pub enum Error {
    LoginFailed,
    ClientVersionOutdated,
    Other(u16, String),
    Unknown(String),
}

fn status_code(s: &str) -> IResult<&str, (u16, &str)> {
    separated_pair(
        map(
            take_while_m_n(3, 3, |c: char| c.is_ascii_digit()),
            |s: &str| s.parse().unwrap(),
        ),
        tag(" "),
        take_till1(|c| c == '\n'),
    )(s)
}

impl Error {
    pub fn parse_from(s: &str) -> Self {
        map(status_code, |(code, msg)| match code {
            500 => Self::LoginFailed,
            503 => Self::ClientVersionOutdated,
            _ => Self::Other(code, msg.to_owned()),
        })(s)
        .map(|(_s, e)| e)
        .unwrap_or_else(|_| Error::Unknown(s.to_owned()))
    }
}

parser!(LoginAccepted           { "200 " {session_key: String} " LOGIN ACCEPTED\n" });
parser!(LoginAcceptedNewVersion { "201 " {session_key: String} " LOGIN ACCEPTED - NEW VERSION AVAILABLE\n" });
parser!(LoggedOut               { "203 LOGGED OUT\n" });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn some_errors() {
        assert_eq!(
            Error::parse_from("500 LOGIN FAILED"),
            Error::LoginFailed,
        );
        assert_eq!(
            Error::parse_from("503 CLIENT VERSION OUTDATED"),
            Error::ClientVersionOutdated,
        );
    }
}
