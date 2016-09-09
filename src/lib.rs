//! A machine learning library for supervised regression trainings
//!
//! This library wants to enable its user to write teachers
//! independent of the model trained or the cost function tried to
//! minimize.
//! Consequently its two main traits are currently `Model`, `Cost`
//! and `Teacher`.

#![warn(missing_docs)]

extern crate num;

use std::iter::IntoIterator;
use num::{Zero, One, Float};

/// A Model is defines how to predict a target from an input
///
/// A model usually depends on several coefficents whose values
/// are derived using a training algorithm
pub trait Model : Clone{
    /// Input features
    type Input;
    /// Target type
    type Target : Float;

    /// Predicts a target for the inputs based on the internal coefficents
    fn predict(&self, &Self::Input) -> Self::Target;

    /// The number of internal coefficents this model depends on
    fn num_coefficents(&self) -> usize;

    /// Value predict derived by the n-th `coefficent` at `input`
    fn gradient(&self, coefficent : usize, input : &Self::Input) -> Self::Target;

    /// Mutable reference to the n-th `coefficent`
    fn coefficent(& mut self, coefficent : usize) -> & mut Self::Target;
}

/// Cost functions those value is supposed be minimized by the training algorithm
pub trait Cost{

    /// Error type used by the cost function
    ///
    /// Usually `f64` or `f32`
    type Error : Float;

    /// Value of the cost function derived by the n-th coefficent at x expressed in Error(x) and dY(x)/dx
    ///
    /// This method is called by SGD based training algorithm in order to
    /// determine the delta of the coefficents
    fn gradient(&self, prediction : Self::Error, truth : Self::Error, gradient_error_by_coefficent : Self::Error) -> Self::Error;
}

/// `Teachers` are used to train `Models`, based on events and a `Cost` function
pub trait Teacher<M : Model>{

    /// Changes `model`s coefficents so they minimize the `cost` function (hopefully)
    fn teach_event<C>(&self, cost : &C, model : &mut M, features : &M::Input, truth : M::Target)
        where C : Cost<Error=M::Target>;
}

/// Teaches `model` all events in `history`
pub fn teach_history<M, C, T, H>(teacher : &T, cost : &C, model : &mut M, history : H)
    where M : Model,
    C : Cost<Error=M::Target>,
    T : Teacher<M>,
    H : IntoIterator<Item=(M::Input, M::Target)>
{
    for (features, truth) in history{

        teacher.teach_event(cost, model, &features, truth);
    }
}

/// Changes all coefficents of model based on their derivation of the cost function at features
///
/// Will not get stuck on saddle points as easily as a plain SGD and will converge quicker in general.
/// A good default for `inertia` is 0.9
pub fn inert_gradient_descent_step<C, M>(
    cost : &C,
    model : &mut M,
    features : &M::Input,
    truth : M::Target,
    learning_rate : M::Target,
    inertia : M::Target,
    velocity : & mut Vec<M::Target>
)
    where C : Cost, M : Model<Target=C::Error>
{
    let inv_inertia = M::Target::one() - inertia;
    let prediction = model.predict(&features);

    for ci in 0..model.num_coefficents(){

        velocity[ci] = inertia * velocity[ci] - inv_inertia * learning_rate * cost.gradient(prediction, truth, model.gradient(ci, features));
        *model.coefficent(ci) = *model.coefficent(ci) + velocity[ci];
    }
}

/// Applies a plain SGD training step to model once for every event in history using a constant learning rate
pub fn stochastic_gradient_descent<C, M, H>(cost : &C, start : M, history : H, learning_rate : M::Target) -> M
    where C : Cost,
    M : Model<Target=C::Error>,
    H : Iterator<Item=(M::Input, M::Target)>
{

    let training = train::GradientDescent{ learning_rate : learning_rate };
    let mut next = start.clone();
    for (features, truth) in history{

        training.teach_event(cost, & mut next, &features, truth);
    }

    next
}

/// SGD tranining with constant learning rate and velocity
///
/// Velocity avoids being stuck on saddle points during optimization
/// A good default for `inertia` is 0.9
pub fn inert_stochastic_gradient_descent<C, M, H>(
    cost : &C,
    start : M,
    history : H,
    learning_rate : M::Target,
    inertia : M::Target
) -> M
    where C : Cost,
    M : Model<Target=C::Error>,
    H : Iterator<Item=(M::Input, M::Target)>
{

    let mut velocity = Vec::new();
    velocity.resize(start.num_coefficents(), M::Target::zero());
    let mut next = start.clone();
    for (features, truth) in history{

        inert_gradient_descent_step(cost, & mut next, &features, truth, learning_rate, inertia, & mut velocity);
    }

    next
}

/// Implementations of `Model` trait
pub mod model;
/// Implementations of `Cost` trait
pub mod cost;
/// Teachers describe how the coefficents of a `Model` change based on `Cost` functions and history
pub mod train;
/// Defines linear algebra traits used for some model parameters
pub mod linear_algebra;

#[cfg(test)]
mod tests {

    #[test]
    fn estimate_median() {

        use model::Constant;
        use cost::LeastAbsoluteDeviation;
        use train::GradientDescent;
        use Teacher;

        let features = ();
        let history = [1.0, 3.0, 4.0, 7.0, 8.0, 11.0, 29.0]; //median is seven

        let cost = LeastAbsoluteDeviation{};
        let mut model = Constant::new(0.0);

        let learning_rate_start = 0.9;
        let decay = 9;

        for (count_step, &truth) in history.iter().cycle().take(150).enumerate(){

            let training = GradientDescent{ learning_rate: learning_rate_start / ( 1.0 + count_step as f64 /decay as f64) as f64 };
            training.teach_event(&cost, &mut model, &features, truth);
            println!("model: {:?}, learning_rate: {:?}", model, training.learning_rate);
        }

        assert!(model.c < 7.1);
        assert!(model.c > 6.9);
    }

    #[test]
    fn estimate_mean() {

        use model::Constant;
        use cost::LeastSquares;
        use train::GradientDescent;
        use Teacher;

        let features = ();
        let history = [1f64, 3.0, 4.0, 7.0, 8.0, 11.0, 29.0]; //mean is 9

        let cost = LeastSquares{};
        let mut model = Constant::new(0.0);

        let learning_rate_start = 0.3;
        let decay = 4;

        for (count_step, &truth) in history.iter().cycle().take(100).enumerate(){

        let training = GradientDescent{ learning_rate: learning_rate_start / ( 1.0 + count_step as f64 /decay as f64) as f64 };
            training.teach_event(&cost, &mut model, &features, truth);
            println!("model: {:?}, learning_rate: {:?}", model, training.learning_rate);
        }

        assert!(model.c < 9.1);
        assert!(model.c > 8.9);
    }

    #[test]
    fn linear_stochastic_gradient_descent() {

        use cost::LeastSquares;
        use model::Linear;
        use train::GradientDescent;
        use teach_history;

        let history = [(0f64, 3f64), (1.0, 4.0), (2.0, 5.0)];

        let mut model = Linear{m : 0.0, c : 0.0};

        let teacher = GradientDescent{ learning_rate : 0.2 };

        let cost = LeastSquares{};
        teach_history(&teacher, &cost, &mut model, history.iter().cycle().take(20).cloned());

        assert!(model.m < 1.1);
        assert!(model.m > 0.9);
        assert!(model.c < 3.1);
        assert!(model.c > 2.9);
    }

    #[test]
    fn linear_stochastic_gradient_descent_iter() {

        use model::Linear;
        use cost::LeastSquares;
        use train::GradientDescent;
        use Teacher;

        let history = [(0f64, 3f64), (1.0, 4.0), (2.0, 5.0)];

        let cost = LeastSquares{};
        let mut model = Linear{m : 0.0, c : 0.0};

        let training = GradientDescent{ learning_rate : 0.2 };

        for &(features, truth) in history.iter().cycle().take(20){

            training.teach_event(&cost, &mut model, &features, truth);
            println!("model: {:?}", model);
        }

        assert!(model.m < 1.1);
        assert!(model.m > 0.9);
        assert!(model.c < 3.1);
        assert!(model.c > 2.9);
    }

    #[test]
    fn linear_sgd_2d(){
        use cost::LeastSquares;
        use model::Linear;
        use inert_stochastic_gradient_descent;

        let history = [([0.0, 7.0], 17.0), ([1.0, 2.0], 8.0), ([2.0, -2.0], 1.0)];

        let start = Linear{m : [0.0, 0.0], c : 0.0};

        let learning_rate = 0.1;

        let cost = LeastSquares{};
        let model = inert_stochastic_gradient_descent(
            &cost, start,
            history.iter().cycle().take(15000).cloned(),
            learning_rate, 0.9
        );

        println!("{:?}", model);

        assert!(model.m[0] < 1.1);
        assert!(model.m[0] > 0.9);
        assert!(model.m[1] < 2.1);
        assert!(model.m[1] > 1.9);
        assert!(model.c < 3.1);
        assert!(model.c > 2.9);
    }

    #[test]
    fn logistic_sgd_2d_least_squares(){
        use cost::LeastSquares;
        use model::{Logicstic, Linear};
        use train::GradientDescent;
        use teach_history;

        use Model;

        let history = [
            ([2.7, 2.5], 0.0),
            ([1.4, 2.3], 0.0),
            ([3.3, 4.4], 0.0),
            ([1.3, 1.8], 0.0),
            ([3.0, 3.0], 0.0),
            ([7.6, 2.7], 1.0),
            ([5.3, 2.0], 1.0),
            ([6.9, 1.7], 1.0),
            ([8.6, -0.2], 1.0),
            ([7.6, 3.5], 1.0)
        ];

        let mut model = Logicstic{ linear: Linear{m : [0.0, 0.0], c : 0.0}};
        let teacher = GradientDescent{ learning_rate : 0.3 };
        let cost = LeastSquares{};

        teach_history(
            &teacher, &cost, &mut model,
            history.iter().cycle().take(40).cloned(),
        );

        println!("{:?}", model.linear);

        let classification_errors = history.iter()
            .map(|&(input, truth)| model.predict(&input).round() == truth)
            .fold(0, |errors, correct| if correct { errors } else { errors + 1 });

        assert_eq!(0, classification_errors);
    }

        #[test]
    fn logistic_sgd_2d_max_likelihood(){
        use cost::MaxLikelihood;
        use model::{Logicstic, Linear};
        use train::GradientDescent;
        use teach_history;
        use Model;

        let history = [
            ([2.7, 2.5], 0.0),
            ([1.4, 2.3], 0.0),
            ([3.3, 4.4], 0.0),
            ([1.3, 1.8], 0.0),
            ([3.0, 3.0], 0.0),
            ([7.6, 2.7], 1.0),
            ([5.3, 2.0], 1.0),
            ([6.9, 1.7], 1.0),
            ([8.6, -0.2], 1.0),
            ([7.6, 3.5], 1.0)
        ];

        let mut model = Logicstic{ linear: Linear{m : [0.0, 0.0], c : 0.0}};
        let teacher = GradientDescent{ learning_rate : 0.3 };
        let cost = MaxLikelihood{};

        teach_history(
            &teacher, &cost, &mut model,
            history.iter().cycle().take(20).cloned(),
        );

        println!("{:?}", model.linear);

        let classification_errors = history.iter()
            .map(|&(input, truth)| model.predict(&input).round() == truth)
            .fold(0, |errors, correct| if correct { errors } else { errors + 1 });

        assert_eq!(0, classification_errors);
    }
}
