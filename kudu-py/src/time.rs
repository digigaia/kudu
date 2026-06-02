use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule(name = "time", submodule)]
pub mod kudu_time {
    use std::string::ToString;

    use pyo3::prelude::*;
    use pyo3::exceptions::PyValueError;
    use pyo3::types::{PyDateAccess, PyDateTime, PyString, PyTimeAccess};

    use chrono::{DateTime, Datelike, NaiveDateTime, Timelike, Utc};

    use kudu::{ABISerializable, TimePoint, TimePointSec};

    use crate::util::{
        gen_bytes_conversion, gen_default_repr, gen_default_str, gen_convert_getters, value_err
    };

    // -----------------------------------------------------------------------------
    //     TimePoint
    // -----------------------------------------------------------------------------

    #[pyclass(name = "TimePoint", module = "kudu.time")]
    pub struct PyTimePoint(pub TimePoint);

    gen_default_repr!("PyTimePoint");
    gen_default_str!("PyTimePoint");
    gen_bytes_conversion!("PyTimePoint");
    gen_convert_getters!("PyTimePoint", ".to_datetime().date_naive()", "i32", ["year"]);
    gen_convert_getters!("PyTimePoint", ".to_datetime().date_naive()", "u32", ["month", "day"]);
    gen_convert_getters!("PyTimePoint", ".to_datetime()", "u32", ["hour", "minute", "second"]);

    #[pymethods]
    impl PyTimePoint {
        #[new]
        fn new<'py>(dt: &Bound<'py, PyAny>) -> PyResult<Self> {
            if let Ok(dt) = dt.cast::<PyString>() {
                Ok(PyTimePoint(dt.to_str()?.parse().map_err(value_err)?))
            }
            else if let Ok(dt) = dt.cast::<PyDateTime>() {
                // FIXME: only no-tz or utc
                Ok(PyTimePoint(TimePoint::new(dt.get_year(),
                                              dt.get_month().into(),
                                              dt.get_day().into(),
                                              dt.get_hour().into(),
                                              dt.get_minute().into(),
                                              dt.get_second().into(),
                                              dt.get_microsecond() / 1000)
                                  .unwrap())) // if it's a valid datetime, it should also be a valid TimePoint
            }
            else if let Ok((y, m, d, h, min, s, milli)) = dt.extract::<(i32, u32, u32, u32, u32, u32, u32)>() {
                match TimePoint::new(y, m, d, h, min, s, milli) {
                    Ok(tp) => Ok(PyTimePoint(tp)),
                    Err(e) => Err(value_err(e)),
                }
            }
            else {
                 Err(PyValueError::new_err("need a string, a datetime or a tuple (year, month, day, hour, minute, second, millisecond)"))
            }
        }

        #[getter]
        pub fn get_milli(&self) -> u32 {
            // use `.into()` to ensure we cover all infallible conversions
            self.0.to_datetime().nanosecond() / 1_000_000
        }

        fn __eq__<'py>(&self, other: &Bound<'py, PyAny>) -> bool {
            // compare using an object of the same type
            if let Ok(dt) = other.cast::<PyTimePoint>() {
                return self.0 == dt.borrow().0;
            }
            // compare using a python string
            if let Ok(dt) = other.extract::<&str>() {
                return self.0.to_string() == dt;
            }
            // compare using a python datetime
            if let Ok(dt) = other.extract::<DateTime<Utc>>() {
                return self.0.to_datetime() == dt;
            }
            // also allow datetimes without timezone info, assume they are UTC
            if let Ok(dt) = other.extract::<NaiveDateTime>() {
                return self.0.to_datetime().naive_utc() == dt;
            }
            false
        }

        fn to_datetime(&self) -> DateTime<Utc> {
            self.0.to_datetime()
        }

    }


    // -----------------------------------------------------------------------------
    //     TimePointSec
    // -----------------------------------------------------------------------------

    #[pyclass(name = "TimePointSec", module = "kudu.time")]
    pub struct PyTimePointSec(pub TimePointSec);

    gen_default_repr!("PyTimePointSec");
    gen_default_str!("PyTimePointSec");
    gen_bytes_conversion!("PyTimePointSec");
    gen_convert_getters!("PyTimePointSec", ".to_datetime().date_naive()", "i32", ["year"]);
    gen_convert_getters!("PyTimePointSec", ".to_datetime().date_naive()", "u32", ["month", "day"]);
    gen_convert_getters!("PyTimePointSec", ".to_datetime()", "u32", ["hour", "minute", "second"]);

    #[pymethods]
    impl PyTimePointSec {
        #[new]
        fn new<'py>(dt: &Bound<'py, PyAny>) -> PyResult<Self> {
            if let Ok(dt) = dt.cast::<PyString>() {
                Ok(PyTimePointSec(dt.to_str()?.parse().map_err(value_err)?))
            }
            else if let Ok(dt) = dt.cast::<PyDateTime>() {
                // FIXME: only no-tz or utc
                Ok(PyTimePointSec(TimePointSec::new(dt.get_year(),
                                                    dt.get_month().into(),
                                                    dt.get_day().into(),
                                                    dt.get_hour().into(),
                                                    dt.get_minute().into(),
                                                    dt.get_second().into())
                                  .unwrap())) // if it's a valid datetime, it should also be a valid TimePointSec
            }
            else if let Ok((y, m, d, h, min, s)) = dt.extract::<(i32, u32, u32, u32, u32, u32)>() {
                match TimePointSec::new(y, m, d, h, min, s) {
                    Ok(tp) => Ok(PyTimePointSec(tp)),
                    Err(e) => Err(value_err(e)),
                }
            }
            else {
                 Err(PyValueError::new_err("need a string, a datetime or a tuple (year, month, day, hour, minute, second)"))
            }
        }

        fn __eq__<'py>(&self, other: &Bound<'py, PyAny>) -> bool {
            // compare using an object of the same type
            if let Ok(dt) = other.cast::<PyTimePointSec>() {
                return self.0 == dt.borrow().0;
            }
            // compare using a python string
            if let Ok(dt) = other.extract::<&str>() {
                return self.0.to_string() == dt;
            }
            // compare using a python datetime
            if let Ok(dt) = other.extract::<DateTime<Utc>>() {
                return self.0.to_datetime() == dt;
            }
            // also allow datetimes without timezone info, assume they are UTC
            if let Ok(dt) = other.extract::<NaiveDateTime>() {
                return self.0.to_datetime().naive_utc() == dt;
            }
            false
        }

        fn to_datetime(&self) -> DateTime<Utc> {
            self.0.to_datetime()
        }

    }

}
