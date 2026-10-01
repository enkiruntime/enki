use super::context::IngressContext;
use super::descriptor::ArgDescriptor;
use super::traits::{GpuType, NamArgs};

macro_rules! impl_nam_args {
    ($($T:ident),*) => {
        impl<$($T: GpuType),*> NamArgs for ($($T,)*) {
            #[allow(non_snake_case)]
            #[allow(unused_assignments)]
            fn collect_all<'a>(self) -> IngressContext<'a> {
                let mut ctx = IngressContext::new();
                let ($($T,)*) = self;

                let mut arg_idx = 0;
                $(
                    ctx.current_arg_index = arg_idx;
                    $T.collect(&mut ctx);
                    arg_idx += 1;
                )*

                ctx
            }

            fn describe_all() -> Vec<ArgDescriptor> {
                vec![
                    $(<$T as GpuType>::describe()),*
                ]
            }
        }
    };
}

impl_nam_args!(A);
impl_nam_args!(A, B);
impl_nam_args!(A, B, C);
impl_nam_args!(A, B, C, D);
impl_nam_args!(A, B, C, D, E);
impl_nam_args!(A, B, C, D, E, F);
impl_nam_args!(A, B, C, D, E, F, G);
impl_nam_args!(A, B, C, D, E, F, G, H);
impl_nam_args!(A, B, C, D, E, F, G, H, I);
impl_nam_args!(A, B, C, D, E, F, G, H, I, J);
impl_nam_args!(A, B, C, D, E, F, G, H, I, J, K);
impl_nam_args!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_nam_args!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_nam_args!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_nam_args!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_nam_args!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
